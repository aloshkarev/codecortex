use cortex_vector::{
    Embedder, HybridResult, HybridSearch, LanceStore, MetadataValue, OllamaEmbedder,
    OpenAIEmbedder, SearchType, VectorDocument, VectorMetadata, VectorStore,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const CODE_EXTENSIONS: &[&str] = &[
    "rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "rb", "c", "cpp", "h", "hpp", "cs", "php",
    "swift", "kt", "kts", "json", "sh", "bash", "zsh",
];

pub struct VectorService {
    store: Arc<LanceStore>,
    embedder: Arc<dyn Embedder>,
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
    pub async fn from_env() -> Result<Self, String> {
        let store_path = vector_store_path();
        let store = Arc::new(
            LanceStore::open(&store_path)
                .await
                .map_err(|e| format!("open vector store failed: {e}"))?,
        );

        let embedder: Arc<dyn Embedder> = if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            let mut openai = OpenAIEmbedder::new(api_key);
            if let Ok(base_url) = std::env::var("OPENAI_BASE_URL") {
                openai = openai.with_base_url(base_url);
            }
            Arc::new(openai)
        } else {
            let mut ollama = if let Ok(model) = std::env::var("CORTEX_OLLAMA_EMBED_MODEL") {
                OllamaEmbedder::with_model(model)
            } else {
                OllamaEmbedder::new()
            };
            if let Ok(base_url) = std::env::var("CORTEX_OLLAMA_BASE_URL") {
                ollama = ollama.with_base_url(base_url);
            }
            Arc::new(ollama)
        };

        Ok(Self { store, embedder })
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

    pub async fn search(
        &self,
        request: VectorSearchRequest<'_>,
    ) -> Result<Vec<HybridResult>, String> {
        let hybrid = HybridSearch::new(self.store.clone(), self.embedder.clone());
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
        let hybrid = HybridSearch::new(self.store.clone(), self.embedder.clone());
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
    ) -> Result<VectorIndexResult, String> {
        let mut files = Vec::new();
        collect_code_files(root_path, &mut files).map_err(|e| format!("walk repo failed: {e}"))?;

        let mut scanned_files = 0usize;
        let mut skipped_files = 0usize;
        let mut documents = Vec::new();
        for file in files {
            scanned_files += 1;
            match build_file_document(&file, repository, branch, revision) {
                Ok(Some(doc)) => documents.push(doc),
                Ok(None) => skipped_files += 1,
                Err(_) => skipped_files += 1,
            }
        }

        let hybrid = HybridSearch::new(self.store.clone(), self.embedder.clone());
        let indexed_documents = hybrid
            .index_documents(documents)
            .await
            .map_err(|e| format!("vector index repository failed: {e}"))?;
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
    if !file_path.is_file() || !is_code_file(file_path) {
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

fn collect_code_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name == ".git" || name == "node_modules" || name == "target" {
                continue;
            }
            collect_code_files(&path, out)?;
        } else if path.is_file() && is_code_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_code_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    CODE_EXTENSIONS.contains(&ext)
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

fn vector_store_path() -> PathBuf {
    if let Ok(path) = std::env::var("CORTEX_VECTOR_STORE_PATH") {
        return PathBuf::from(path);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cortex/vectors")
}

#[cfg(test)]
mod tests {
    use super::{is_code_file, language_from_path};
    use std::path::Path;

    #[test]
    fn recognizes_code_files() {
        assert!(is_code_file(Path::new("src/main.rs")));
        assert!(is_code_file(Path::new("app.ts")));
        assert!(is_code_file(Path::new("scripts/build.sh")));
        assert!(is_code_file(Path::new("build.gradle.kts")));
        assert!(is_code_file(Path::new("package.json")));
        assert!(!is_code_file(Path::new("README.md")));
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
