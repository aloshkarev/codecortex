use cortex_core::{
    CortexConfig, CortexIgnoreOptions, CortexIgnoreWalker, Language, ProjectConfig,
    default_global_cortexignore_path, ensure_cortexignore_template,
};
use cortex_vector::{
    Embedder, FallbackEmbedder, HashEmbedder, HybridResult, HybridSearch, LanceStore,
    MetadataValue, OllamaEmbedder, OpenAIEmbedder, SearchType, StaticEmbedder, VectorDocument,
    VectorMetadata, VectorStore,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct VectorService {
    store: Arc<LanceStore>,
    embedder: Arc<dyn Embedder>,
    use_rrf_fusion: bool,
}

#[derive(Debug, Clone, Default)]
pub struct VectorSearchFilters<'a> {
    pub repository: Option<&'a str>,
    pub path: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub language: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct VectorSearchRequest<'a> {
    pub query: &'a str,
    pub search_type: SearchType,
    pub k: usize,
    pub filters: VectorSearchFilters<'a>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VectorIndexResult {
    pub indexed_documents: usize,
    pub scanned_files: usize,
    pub skipped_files: usize,
}

impl VectorService {
    /// Build an embedder from `~/.cortex/config.toml` (and env overrides).
    ///
    /// Respects `[llm] provider`: when set to `"ollama"`, uses Ollama even if `OPENAI_API_KEY`
    /// is set in the environment (previously the env key always forced OpenAI).
    fn use_rrf_fusion(config: &CortexConfig) -> bool {
        config.vector.hybrid_fusion.to_ascii_lowercase() != "legacy"
    }

    pub fn build_embedder(config: &CortexConfig) -> Result<Arc<dyn Embedder>, String> {
        if std::env::var("CORTEX_TEST_EMBEDDER").ok().as_deref() == Some("1") {
            return Ok(Arc::new(HashEmbedder::new()));
        }

        let provider = config.llm.provider.trim().to_ascii_lowercase();

        if provider == "test" {
            return Ok(Arc::new(HashEmbedder::new()));
        }

        let use_openai = match provider.as_str() {
            "openai" => true,
            "ollama" | "local" => false,
            // "none" or unknown: preserve legacy behavior (OpenAI if key exists)
            _ => std::env::var("OPENAI_API_KEY").is_ok(),
        };

        let primary: Arc<dyn Embedder> = if use_openai {
            let api_key = config
                .llm
                .openai_api_key
                .clone()
                .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                .ok_or_else(|| {
                    "OpenAI embeddings selected but no API key (set llm.openai_api_key or OPENAI_API_KEY)"
                        .to_string()
                })?;
            let model = config.llm.openai_embedding_model.clone();
            let mut openai = OpenAIEmbedder::with_model(api_key, model);
            if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
                openai = openai.with_base_url(base_url);
            }
            Arc::new(openai)
        } else {
            let model = std::env::var("CORTEX_OLLAMA_EMBED_MODEL")
                .unwrap_or_else(|_| config.llm.ollama_embedding_model.clone().trim().to_string());
            if model.is_empty() {
                return Err(
                    "Ollama embedding model is empty (set llm.ollama_embedding_model or CORTEX_OLLAMA_EMBED_MODEL)"
                        .to_string(),
                );
            }
            let mut ollama = OllamaEmbedder::with_model(model);
            let base = std::env::var("CORTEX_OLLAMA_BASE_URL")
                .unwrap_or_else(|_| config.llm.ollama_base_url.trim().to_string());
            if !base.is_empty() {
                ollama = ollama.with_base_url(base);
            }
            Arc::new(ollama)
        };

        if config.vector.embedding_fallback == "static" {
            Ok(Arc::new(FallbackEmbedder::new(
                primary,
                StaticEmbedder::new(),
            )))
        } else {
            Ok(primary)
        }
    }

    pub fn embedder_label(embedder: &Arc<dyn Embedder>) -> String {
        embedder.model().to_string()
    }

    pub async fn from_config(config: &CortexConfig) -> Result<Self, String> {
        let store_path = vector_store_path(config);
        let store = Arc::new(
            LanceStore::open(&store_path)
                .await
                .map_err(|e| format!("open vector store failed: {e}"))?,
        );
        let embedder = Self::build_embedder(config)?;
        Ok(Self {
            store,
            embedder,
            use_rrf_fusion: Self::use_rrf_fusion(config),
        })
    }

    fn hybrid_search(&self) -> HybridSearch {
        HybridSearch::with_fusion(
            self.store.clone(),
            self.embedder.clone(),
            self.use_rrf_fusion,
        )
    }

    /// Same as [`Self::from_config`] using `CortexConfig::load()` (or defaults if load fails).
    pub async fn from_env() -> Result<Self, String> {
        let config = CortexConfig::load().unwrap_or_default();
        Self::from_config(&config).await
    }

    pub fn embedder(&self) -> &Arc<dyn Embedder> {
        &self.embedder
    }

    pub async fn health_check(&self) -> Result<bool, String> {
        self.store
            .health_check()
            .await
            .map_err(|e| format!("vector health check failed: {e}"))
    }

    pub async fn total_documents(&self) -> Result<usize, String> {
        self.store
            .count()
            .await
            .map_err(|e| format!("vector count failed: {e}"))
    }

    pub async fn count_documents(&self, repository: Option<&str>) -> Result<usize, String> {
        let filter = repository.map(|repo| {
            let mut filter = HashMap::new();
            filter.insert(
                "repository".to_string(),
                MetadataValue::String(repo.to_string()),
            );
            filter
        });
        match filter {
            Some(f) => self
                .store
                .count_by_filter(f)
                .await
                .map_err(|e| format!("vector count by repository failed: {e}")),
            None => self.total_documents().await,
        }
    }

    pub async fn search(
        &self,
        request: VectorSearchRequest<'_>,
    ) -> Result<Vec<HybridResult>, String> {
        let hybrid = self.hybrid_search();
        let filter = build_metadata_filter(request.filters);

        if filter.is_empty() {
            hybrid
                .search(request.query, request.search_type, request.k)
                .await
                .map_err(|e| format!("vector search failed: {e}"))
        } else if matches!(request.search_type, SearchType::Semantic) {
            hybrid
                .semantic_search_with_filter(request.query, request.k, filter)
                .await
                .map_err(|e| format!("vector search with filter failed: {e}"))
        } else {
            // Keep hybrid/structural semantics even when filters are requested.
            let expanded_k = request.k.saturating_mul(8).max(request.k).min(200);
            let mut results = hybrid
                .search(request.query, request.search_type, expanded_k)
                .await
                .map_err(|e| format!("vector search failed: {e}"))?;
            results.retain(|r| metadata_matches_filter(&r.result.metadata, &filter));
            results.truncate(request.k);
            Ok(results)
        }
    }

    pub async fn delete_repository(&self, repo_path: &str) -> Result<usize, String> {
        let mut filter = HashMap::new();
        filter.insert(
            "repository".to_string(),
            MetadataValue::String(repo_path.to_string()),
        );
        self.store
            .delete_by_filter(filter)
            .await
            .map_err(|e| format!("vector delete repository failed: {e}"))
    }

    pub async fn index_file(
        &self,
        file_path: &Path,
        repository: &str,
        branch: &str,
        revision: &str,
    ) -> Result<VectorIndexResult, String> {
        let Some(document) = build_file_document(file_path, repository, branch, revision)? else {
            return Ok(VectorIndexResult {
                indexed_documents: 0,
                scanned_files: 1,
                skipped_files: 1,
            });
        };
        let hybrid = self.hybrid_search();
        let indexed = hybrid
            .index_documents(vec![document])
            .await
            .map_err(|e| format!("vector index file failed: {e}"))?;
        Ok(VectorIndexResult {
            indexed_documents: indexed,
            scanned_files: 1,
            skipped_files: 0,
        })
    }

    pub async fn index_repository(
        &self,
        root_path: &Path,
        repository: &str,
        branch: &str,
        revision: &str,
        include_paths: Option<&[String]>,
        max_files: Option<usize>,
        cortex_config: &CortexConfig,
    ) -> Result<VectorIndexResult, String> {
        let files = collect_indexable_code_files(root_path, cortex_config, &[])
            .map_err(|e| format!("walk repo failed: {e}"))?;

        let mut scanned_files = 0usize;
        let mut skipped_files = 0usize;
        let mut pending = Vec::new();
        let mut indexed_documents = 0usize;
        const INDEX_CHUNK: usize = 8;

        let hybrid = self.hybrid_search();

        for file in files {
            if let Some(paths) = include_paths {
                let rel = file
                    .strip_prefix(root_path)
                    .unwrap_or(&file)
                    .to_string_lossy()
                    .replace('\\', "/");
                let included = paths.iter().any(|p| {
                    let p = p.trim().trim_start_matches('/').replace('\\', "/");
                    rel == p || rel.starts_with(&format!("{p}/"))
                });
                if !included {
                    continue;
                }
            }
            if let Some(max) = max_files {
                if scanned_files >= max {
                    break;
                }
            }
            scanned_files += 1;
            match build_file_document(&file, repository, branch, revision) {
                Ok(Some(doc)) => {
                    pending.push(doc);
                    if pending.len() >= INDEX_CHUNK {
                        indexed_documents += hybrid
                            .index_documents(std::mem::take(&mut pending))
                            .await
                            .map_err(|e| format!("vector index repository failed: {e}"))?;
                        tracing::info!(
                            indexed_documents,
                            scanned_files,
                            root = %root_path.display(),
                            "vector index progress"
                        );
                    }
                }
                Ok(None) => skipped_files += 1,
                Err(_) => skipped_files += 1,
            }
        }

        if !pending.is_empty() {
            indexed_documents += hybrid
                .index_documents(pending)
                .await
                .map_err(|e| format!("vector index repository failed: {e}"))?;
        }

        Ok(VectorIndexResult {
            indexed_documents,
            scanned_files,
            skipped_files,
        })
    }

    pub async fn embed_query(&self, text: &str) -> Result<Vec<f32>, String> {
        self.embedder
            .embed_query(text)
            .await
            .map_err(|e| format!("embed query failed: {e}"))
    }
}

fn build_file_document(
    file_path: &Path,
    repository: &str,
    branch: &str,
    revision: &str,
) -> Result<Option<VectorDocument>, String> {
    if !file_path.is_file() || Language::from_path(file_path).is_none() {
        return Ok(None);
    }
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("read {} failed: {e}", file_path.display()))?;
    if content.trim().is_empty() {
        return Ok(None);
    }

    let normalized_path = file_path.to_string_lossy().to_string();
    let language = language_from_path(file_path);
    let id = format!("{}:{}:{}:file", repository, normalized_path, revision);
    let metadata = VectorMetadata::code_symbol(&normalized_path, "", "file", language)
        .with_repository(repository.to_string(), branch.to_string())
        .with_extra("revision", serde_json::Value::String(revision.to_string()));
    Ok(Some(VectorDocument::with_metadata(
        id,
        vec![0.0; cortex_vector::EMBEDDING_DIMENSION],
        content,
        metadata,
    )))
}

fn build_metadata_filter(filters: VectorSearchFilters<'_>) -> HashMap<String, MetadataValue> {
    let mut filter = HashMap::new();
    if let Some(repo) = filters.repository {
        filter.insert(
            "repository".to_string(),
            MetadataValue::String(repo.to_string()),
        );
    }
    if let Some(path) = filters.path {
        filter.insert("path".to_string(), MetadataValue::String(path.to_string()));
    }
    if let Some(kind) = filters.kind {
        filter.insert("kind".to_string(), MetadataValue::String(kind.to_string()));
    }
    if let Some(language) = filters.language {
        filter.insert(
            "language".to_string(),
            MetadataValue::String(language.to_string()),
        );
    }
    filter
}

fn metadata_matches_filter(
    metadata: &HashMap<String, MetadataValue>,
    filter: &HashMap<String, MetadataValue>,
) -> bool {
    filter.iter().all(|(key, value)| {
        metadata
            .get(key)
            .is_some_and(|candidate| candidate == value)
    })
}

/// Collect code files under `root` using the same ignore rules as graph indexing.
pub fn collect_indexable_code_files(
    root: &Path,
    config: &CortexConfig,
    extra_excludes: &[String],
) -> cortex_core::Result<Vec<PathBuf>> {
    Ok(collect_indexable_code_files_with_stats(root, config, extra_excludes)?.files)
}

/// Like [`collect_indexable_code_files`] but returns ignore skip statistics.
pub fn collect_indexable_code_files_with_stats(
    root: &Path,
    config: &CortexConfig,
    extra_excludes: &[String],
) -> cortex_core::Result<cortex_core::CollectFilesResult> {
    let scan_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let repo_root = cortex_core::find_git_repository_root(&scan_root)
        .map(|p| p.canonicalize().unwrap_or(p))
        .unwrap_or_else(|| scan_root.clone());

    let _ = ensure_cortexignore_template(&repo_root, &ProjectConfig::default().ignore_patterns);

    let mut policy = config.index_exclude_patterns.clone();
    policy.extend_from_slice(extra_excludes);

    let walker = CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root,
        scan_root: Some(scan_root.clone()),
        global_ignore_path: config
            .global_cortexignore_path
            .clone()
            .or_else(default_global_cortexignore_path),
        respect_gitignore: true,
        respect_cortexignore: true,
        include_hidden: false,
        policy_excludes: policy,
        count_ignored_skips: true,
    });

    walker.collect_files_with_stats(&scan_root, None, |p| Language::from_path(p).is_some())
}

fn language_from_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "jsx" => "javascript",
        "ts" => "typescript",
        "tsx" => "typescript",
        "go" => "go",
        "java" => "java",
        "rb" => "ruby",
        "c" => "c",
        "h" => "c",
        "cpp" => "cpp",
        "hpp" => "cpp",
        "cs" => "csharp",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "json" => "json",
        "sh" | "bash" | "zsh" => "shell",
        _ => "unknown",
    }
}

/// Resolve LanceDB path (`CORTEX_VECTOR_STORE_PATH` env overrides `config.vector.store_path`).
pub fn vector_store_path(config: &CortexConfig) -> PathBuf {
    if let Ok(path) = std::env::var("CORTEX_VECTOR_STORE_PATH") {
        return PathBuf::from(path);
    }
    config.vector.store_path.clone()
}

#[cfg(test)]
mod tests {
    use super::{VectorService, collect_indexable_code_files, language_from_path};
    use cortex_core::{CortexConfig, Language};
    use cortex_vector::{EmbeddingProvider, HashEmbedder};
    use std::path::Path;

    #[test]
    fn build_embedder_uses_ollama_when_config_provider_is_ollama() {
        let mut config = CortexConfig::default();
        config.llm.provider = "ollama".to_string();
        config.llm.ollama_embedding_model = "nomic-embed-text".to_string();
        config.vector.embedding_fallback = "none".to_string();
        let embedder = VectorService::build_embedder(&config).expect("embedder");
        assert_eq!(embedder.provider(), EmbeddingProvider::Ollama);
        assert_eq!(embedder.model(), "nomic-embed-text");
    }

    #[test]
    fn build_embedder_uses_hash_when_test_provider_or_env() {
        let mut config = CortexConfig::default();
        config.llm.provider = "test".to_string();
        let embedder = VectorService::build_embedder(&config).expect("embedder");
        assert_eq!(embedder.provider(), EmbeddingProvider::Test);
        assert_eq!(embedder.model(), HashEmbedder::MODEL);

        unsafe {
            std::env::set_var("CORTEX_TEST_EMBEDDER", "1");
        }
        let embedder_env =
            VectorService::build_embedder(&CortexConfig::default()).expect("embedder");
        assert_eq!(embedder_env.provider(), EmbeddingProvider::Test);
        unsafe {
            std::env::remove_var("CORTEX_TEST_EMBEDDER");
        }
    }

    #[test]
    fn recognizes_code_files() {
        assert!(Language::from_path(Path::new("src/main.rs")).is_some());
        assert!(Language::from_path(Path::new("app.ts")).is_some());
        assert!(Language::from_path(Path::new("scripts/build.sh")).is_some());
        assert!(Language::from_path(Path::new("build.gradle.kts")).is_some());
        assert!(Language::from_path(Path::new("package.json")).is_some());
        assert!(Language::from_path(Path::new("README.md")).is_none());
    }

    #[test]
    fn collect_indexable_code_files_honors_cortexignore() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join(".cortexignore"), "skip.rs\n").unwrap();
        std::fs::write(root.join("skip.rs"), "fn s() {}").unwrap();
        std::fs::write(root.join("keep.rs"), "fn k() {}").unwrap();
        let files = collect_indexable_code_files(root, &CortexConfig::default(), &[]).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("keep.rs"));
    }

    #[test]
    fn detects_language_from_extension() {
        assert_eq!(language_from_path(Path::new("src/lib.rs")), "rust");
        assert_eq!(language_from_path(Path::new("src/app.py")), "python");
        assert_eq!(language_from_path(Path::new("src/main.kt")), "kotlin");
        assert_eq!(language_from_path(Path::new("Package.swift")), "swift");
        assert_eq!(language_from_path(Path::new("package.json")), "json");
        assert_eq!(language_from_path(Path::new("scripts/build.sh")), "shell");
    }
}
