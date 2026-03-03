use crate::build_detector::{BuildDetector, ProjectConfig};
use cortex_core::{
    CodeEdge, CodeNode, CortexError, EdgeKind, EntityKind, IndexedFile, Repository, Result,
};
use cortex_graph::{
    BranchIndexRecord, GraphClient, NodeWriter, create_branch_index, is_branch_index_current,
};
use cortex_parser::ParserRegistry;
use ignore::WalkBuilder;
use rayon::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tracing::{Level, info, instrument, span, warn};

/// Configuration for indexing operations
#[derive(Debug, Clone, Serialize)]
pub struct IndexConfig {
    /// Maximum time allowed for indexing (0 = no timeout)
    pub timeout_secs: u64,
    /// Batch size for writing nodes/edges
    pub batch_size: usize,
    /// Maximum number of files to index (0 = unlimited)
    pub max_files: usize,
    /// Enable progress reporting
    pub progress_callback: Option<String>,
    /// Branch name for branch-aware indexing
    pub branch: Option<String>,
    /// Commit hash for the branch
    pub commit_hash: Option<String>,
    /// Repository path (for branch-aware indexing)
    pub repository_path: Option<String>,
    /// Skip indexing if branch is already current
    pub skip_if_current: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 300, // 5 minutes default
            batch_size: 1000,
            max_files: 0,
            progress_callback: None,
            branch: None,
            commit_hash: None,
            repository_path: None,
            skip_if_current: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct IndexReport {
    pub scanned_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    pub resolved_calls: usize,
    /// Detected build systems
    pub build_systems: Vec<String>,
    /// Number of compile commands loaded (for C/C++ projects)
    pub compile_commands_loaded: usize,
    /// Number of include paths detected
    pub include_paths_count: usize,
    /// Time taken for indexing
    pub duration_secs: f64,
    /// Whether indexing timed out
    pub timed_out: bool,
    /// Branch that was indexed (if branch-aware)
    pub branch: Option<String>,
    /// Commit hash that was indexed
    pub commit_hash: Option<String>,
    /// Number of symbols indexed
    pub symbol_count: usize,
    /// Whether indexing was skipped (branch already current)
    pub skipped_reason: Option<String>,
}

/// Progress update during indexing
#[derive(Debug, Clone, Serialize)]
pub struct IndexProgress {
    /// Current phase
    pub phase: IndexPhase,
    /// Files processed so far
    pub files_processed: usize,
    /// Total files to process
    pub total_files: usize,
    /// Percentage complete
    pub percent: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum IndexPhase {
    Scanning,
    Parsing,
    WritingNodes,
    WritingEdges,
    Resolving,
    Complete,
}

#[derive(Clone)]
pub struct Indexer {
    client: GraphClient,
    writer: NodeWriter,
    parser_registry: ParserRegistry,
    cache: sled::Db,
    config: IndexConfig,
}

impl Indexer {
    #[instrument(skip(client))]
    pub fn new(client: GraphClient, batch_size: usize) -> Result<Self> {
        Self::with_config(
            client,
            IndexConfig {
                batch_size,
                ..Default::default()
            },
        )
    }

    /// Create an indexer with custom configuration
    #[instrument(skip(client))]
    pub fn with_config(client: GraphClient, config: IndexConfig) -> Result<Self> {
        let cache = sled::open(Self::cache_path()).map_err(|e| CortexError::Io(e.to_string()))?;
        Ok(Self {
            client: client.clone(),
            writer: NodeWriter::new(client, config.batch_size),
            parser_registry: ParserRegistry::new(),
            cache,
            config,
        })
    }

    pub fn cache_path() -> PathBuf {
        if let Ok(path) = std::env::var("CORTEX_CACHE_PATH") {
            return PathBuf::from(path);
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cortex/hashes.db")
    }

    /// Index a path without branch awareness (backward compatible)
    pub async fn index_path<P: AsRef<Path>>(&self, path: P) -> Result<IndexReport> {
        self.index_path_with_options(path, false).await
    }

    /// Index a path with branch awareness
    pub async fn index_path_with_branch<P: AsRef<Path>>(
        &self,
        path: P,
        branch: &str,
        commit_hash: &str,
    ) -> Result<IndexReport> {
        let mut config = self.config.clone();
        config.branch = Some(branch.to_string());
        config.commit_hash = Some(commit_hash.to_string());
        config.repository_path = Some(path.as_ref().display().to_string());

        self.index_path_with_config(path, false, &config).await
    }

    /// Index with timeout support
    pub async fn index_path_with_timeout<P: AsRef<Path>>(
        &self,
        path: P,
        force: bool,
        timeout: Duration,
    ) -> Result<IndexReport> {
        let start = Instant::now();

        // Use tokio::select for timeout
        let result = tokio::select! {
            result = self.index_path_with_options(&path, force) => {
                let mut report = result?;
                report.duration_secs = start.elapsed().as_secs_f64();
                Ok(report)
            },
            _ = tokio::time::sleep(timeout) => {
                Err(CortexError::Timeout(format!(
                    "Indexing timed out after {:?}",
                    timeout
                )))
            }
        };

        result
    }

    pub async fn index_path_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        force: bool,
    ) -> Result<IndexReport> {
        self.index_path_with_config(path, force, &self.config).await
    }

    /// Index with full configuration support including branch awareness
    #[instrument(skip(self, path, force, config), fields(branch = ?config.branch, commit = ?config.commit_hash))]
    async fn index_path_with_config<P: AsRef<Path>>(
        &self,
        path: P,
        force: bool,
        config: &IndexConfig,
    ) -> Result<IndexReport> {
        let span = span!(Level::INFO, "indexing");
        let _enter = span.enter();
        let start = Instant::now();
        let root = normalize_root(path.as_ref());
        info!(
            path = %root.display(),
            force = force,
            branch = ?config.branch.as_deref(),
            "Starting index operation"
        );

        // Get repository path (use config or derive from path)
        let repository_path = config
            .repository_path
            .clone()
            .unwrap_or_else(|| root.display().to_string());

        // Get branch info
        let branch = config.branch.clone();
        let commit_hash = config.commit_hash.clone();

        // Check if we should skip indexing (branch already current)
        if config.skip_if_current
            && let (Some(br), Some(ch)) = (&branch, &commit_hash)
            && is_branch_index_current(&self.client, &repository_path, br, ch).await?
        {
            return Ok(IndexReport {
                scanned_files: 0,
                indexed_files: 0,
                skipped_files: 0,
                resolved_calls: 0,
                build_systems: vec![],
                compile_commands_loaded: 0,
                include_paths_count: 0,
                duration_secs: start.elapsed().as_secs_f64(),
                timed_out: false,
                branch: Some(br.clone()),
                commit_hash: Some(ch.clone()),
                symbol_count: 0,
                skipped_reason: Some("Branch index already current".to_string()),
            });
        }

        // Check if we have a timeout configured
        let timeout_duration = if config.timeout_secs > 0 {
            Some(Duration::from_secs(config.timeout_secs))
        } else {
            None
        };

        // Detect build system and project configuration
        let detector = BuildDetector::new(&root);
        let project_config = detector.detect();

        let repository = Repository {
            path: repository_path.clone(),
            name: project_config.name.clone().unwrap_or_else(|| {
                root.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("repository")
                    .to_string()
            }),
            watched: false,
        };
        self.client.upsert_repository(&repository).await?;

        // Collect files using detected configuration
        let files = collect_source_files_with_config(root.as_path(), &project_config);
        let scanned_files = files.len();

        // Apply max files limit if configured
        let files: Vec<_> = if config.max_files > 0 {
            files.into_iter().take(config.max_files).collect()
        } else {
            files
        };

        // Build compile command index for C/C++ files
        let compile_cmd_index: HashMap<PathBuf, _> = project_config
            .compile_commands
            .iter()
            .map(|cmd| (cmd.file.clone(), cmd))
            .collect();

        // Parse files with progress tracking
        let _files_count = files.len();
        let processed = Arc::new(AtomicUsize::new(0));
        let timed_out = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let parsed: Vec<Result<Option<IndexedFile>>> = files
            .par_iter()
            .map(|path| {
                // Check timeout
                if let Some(timeout) = timeout_duration
                    && start.elapsed() > timeout
                {
                    timed_out.store(true, Ordering::Relaxed);
                    return Err(CortexError::Timeout("Indexing timed out".to_string()));
                }

                let result = self.parse_and_filter_with_config(path, force, &compile_cmd_index);
                processed.fetch_add(1, Ordering::Relaxed);
                result
            })
            .collect();

        let mut indexed = Vec::new();
        let mut skipped_files = 0usize;
        for item in parsed {
            match item {
                Ok(Some(file)) => indexed.push(file),
                Ok(None) => skipped_files += 1,
                Err(e) => {
                    // Log error but continue
                    eprintln!("Warning: Failed to parse file: {}", e);
                    skipped_files += 1;
                }
            }
        }

        // Count symbols
        let symbol_count: usize = indexed.iter().map(|f| f.nodes.len()).sum();

        // Build hierarchy nodes and edges with branch properties
        let mut pass_one_nodes = Vec::new();
        let mut pass_two_edges = Vec::new();
        let mut hierarchy_nodes = vec![];
        let mut hierarchy_edges = vec![];
        let repo_id = format!("repo:{}", repository_path);
        let mut seen_dirs = HashSet::new();

        for file in &indexed {
            let file_path = PathBuf::from(&file.path);
            let file_id = format!("file:{}", file.path);

            hierarchy_nodes.push(CodeNode {
                id: file_id.clone(),
                kind: EntityKind::File,
                name: file_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string(),
                path: Some(file.path.clone()),
                line_number: Some(1),
                lang: Some(file.language),
                source: None,
                docstring: None,
                properties: build_branch_properties(&branch, &repository_path),
            });
            hierarchy_edges.push(CodeEdge {
                from: repo_id.clone(),
                to: file_id.clone(),
                kind: EdgeKind::Contains,
                properties: HashMap::new(),
            });

            // Build directory chain
            let chain = directory_chain(&root, &file_path);
            if chain.is_empty() {
                continue;
            }

            // Create directory nodes
            for dir in &chain {
                let dir_key = dir.display().to_string();
                if seen_dirs.insert(dir_key.clone()) {
                    hierarchy_nodes.push(CodeNode {
                        id: format!("dir:{dir_key}"),
                        kind: EntityKind::Directory,
                        name: dir
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or_default()
                            .to_string(),
                        path: Some(dir_key.clone()),
                        line_number: Some(1),
                        lang: None,
                        source: None,
                        docstring: None,
                        properties: build_branch_properties(&branch, &repository_path),
                    });
                }
            }

            // Create directory edges
            if let Some(first_dir) = chain.first() {
                let first_dir_str = first_dir.display().to_string();
                hierarchy_edges.push(CodeEdge {
                    from: repo_id.clone(),
                    to: format!("dir:{first_dir_str}"),
                    kind: EdgeKind::Contains,
                    properties: HashMap::new(),
                });

                for window in chain.windows(2) {
                    let from = format!("dir:{}", window[0].display());
                    let to = format!("dir:{}", window[1].display());
                    hierarchy_edges.push(CodeEdge {
                        from,
                        to,
                        kind: EdgeKind::Contains,
                        properties: HashMap::new(),
                    });
                }

                if let Some(last_dir) = chain.last() {
                    let last_dir_str = format!("dir:{}", last_dir.display());
                    hierarchy_edges.push(CodeEdge {
                        from: last_dir_str,
                        to: file_id,
                        kind: EdgeKind::Contains,
                        properties: HashMap::new(),
                    });
                }
            }
        }

        // Collect all nodes and edges, adding branch properties
        for file in &indexed {
            // Add branch properties to all nodes
            let mut nodes_with_branch: Vec<CodeNode> = file.nodes.clone();
            for node in &mut nodes_with_branch {
                for (k, v) in build_branch_properties(&branch, &repository_path) {
                    node.properties.insert(k, v);
                }
            }
            pass_one_nodes.extend(nodes_with_branch);
        }
        for file in &indexed {
            pass_two_edges.extend(file.edges.clone());
        }

        pass_one_nodes.extend(hierarchy_nodes);
        pass_two_edges.extend(hierarchy_edges);

        self.writer.write_nodes(&pass_one_nodes).await?;

        // Resolve call targets
        let call_targets: HashSet<(String, String)> = pass_two_edges
            .iter()
            .filter_map(|edge| {
                if !matches!(edge.kind, EdgeKind::Calls) || !edge.to.starts_with("call_target:") {
                    return None;
                }
                let name = edge.to.trim_start_matches("call_target:").to_string();
                Some((edge.to.clone(), name))
            })
            .collect();
        for (id, name) in &call_targets {
            self.client.upsert_call_target(id, name).await?;
        }

        self.writer.write_edges(&pass_two_edges).await?;
        let resolved_calls = self
            .client
            .resolve_call_targets(&repository_path)
            .await
            .unwrap_or(0);

        // Build system info for report
        let build_systems: Vec<String> = project_config
            .build_systems
            .iter()
            .map(|b| b.to_string())
            .collect();

        let timed_out = timed_out.load(Ordering::Relaxed);

        // Create branch index record if branch-aware indexing
        if let (Some(br), Some(ch)) = (&branch, &commit_hash) {
            let record = BranchIndexRecord::new(
                &repository_path,
                br,
                ch,
                indexed.len(),
                symbol_count,
                start.elapsed().as_millis() as u64,
            );
            create_branch_index(&self.client, &record).await?;
        }

        Ok(IndexReport {
            scanned_files,
            indexed_files: indexed.len(),
            skipped_files,
            resolved_calls,
            build_systems,
            compile_commands_loaded: project_config.compile_commands.len(),
            include_paths_count: project_config.include_paths.len(),
            duration_secs: start.elapsed().as_secs_f64(),
            timed_out,
            branch,
            commit_hash,
            symbol_count,
            skipped_reason: None,
        })
    }

    /// Get current indexing progress
    pub fn get_progress(&self, processed: usize, total: usize) -> IndexProgress {
        let percent = if total > 0 {
            (processed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        IndexProgress {
            phase: IndexPhase::Parsing,
            files_processed: processed,
            total_files: total,
            percent,
        }
    }

    fn parse_and_filter_with_config(
        &self,
        path: &Path,
        force: bool,
        compile_cmd_index: &HashMap<PathBuf, &crate::build_detector::CompileCommand>,
    ) -> Result<Option<IndexedFile>> {
        let source = std::fs::read_to_string(path).map_err(|e| CortexError::Io(e.to_string()))?;
        let hash = file_hash(&source);
        if !force
            && self
                .cache
                .get(path.to_string_lossy().as_bytes())
                .map_err(|e| CortexError::Io(e.to_string()))?
                .as_deref()
                == Some(hash.as_bytes())
        {
            return Ok(None);
        }

        let parser = self.parser_registry.parser_for_path(path)?;

        // For C/C++ files, check if we have compile commands
        let _defines: HashMap<String, Option<String>> = compile_cmd_index
            .get(path)
            .map(|cmd| cmd.defines())
            .unwrap_or_default();

        // Note: In a full implementation, we would pass defines to the parser
        // For now, we just parse normally - the defines are captured for future use
        let parsed = parser.parse(&source, path)?;
        self.cache
            .insert(path.to_string_lossy().as_bytes(), hash.as_bytes())
            .map_err(|e| CortexError::Io(e.to_string()))?;

        let language = cortex_core::Language::from_path(path)
            .ok_or_else(|| CortexError::UnsupportedLanguage(path.display().to_string()))?;
        Ok(Some(IndexedFile {
            path: path.display().to_string(),
            language,
            content_hash: hash,
            nodes: parsed.nodes,
            edges: parsed.edges,
        }))
    }
}

/// Build branch properties map for node properties
fn build_branch_properties(
    branch: &Option<String>,
    repository_path: &str,
) -> HashMap<String, String> {
    let mut props = HashMap::new();
    if let Some(br) = branch {
        props.insert("branch".to_string(), br.clone());
    }
    props.insert("repository_path".to_string(), repository_path.to_string());
    props
}

fn normalize_root(path: &Path) -> PathBuf {
    if path.is_file() {
        path.parent().unwrap_or(path).to_path_buf()
    } else {
        path.to_path_buf()
    }
}

fn directory_chain(root: &Path, file_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = file_path.parent() else {
        return Vec::new();
    };
    let rel_parent = parent.strip_prefix(root).unwrap_or(parent);
    let mut chain = Vec::new();
    let mut current = root.to_path_buf();
    for component in rel_parent.components() {
        current.push(component);
        if current != root {
            chain.push(current.clone());
        }
    }
    chain
}

fn collect_source_files_with_config(path: &Path, config: &ProjectConfig) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
        return files;
    }

    let mut builder = WalkBuilder::new(path);
    builder.hidden(false).git_ignore(true).git_exclude(true);

    for entry in builder.build().flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }

        // Skip files matching exclude patterns from build detection
        let entry_path = entry.path();
        let path_str = entry_path.to_string_lossy();

        let should_exclude = config.exclude_patterns.iter().any(|pattern| {
            if pattern.ends_with("/**") {
                let dir = &pattern[..pattern.len() - 3];
                // Check if any component matches
                entry_path
                    .components()
                    .any(|c| c.as_os_str() == std::ffi::OsStr::new(dir))
            } else if pattern.starts_with("*.") {
                let ext = &pattern[1..]; // ".pyc" from "*.pyc"
                entry_path
                    .extension()
                    .map(|e| format!(".{}", e.to_string_lossy()) == ext)
                    .unwrap_or(false)
            } else {
                path_str.contains(pattern)
            }
        });

        if should_exclude {
            continue;
        }

        // Check for supported language
        if cortex_core::Language::from_path(entry_path).is_some() {
            files.push(entry_path.to_path_buf());
        }
    }

    files
}

pub fn file_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_branch_properties() {
        let branch = Some("main".to_string());
        let props = build_branch_properties(&branch, "/path/to/repo");
        assert_eq!(props.get("branch"), Some(&"main".to_string()));
        assert_eq!(
            props.get("repository_path"),
            Some(&"/path/to/repo".to_string())
        );
    }

    #[test]
    fn test_build_branch_properties_none() {
        let branch = None;
        let props = build_branch_properties(&branch, "/path/to/repo");
        assert_eq!(props.get("branch"), None);
        assert_eq!(
            props.get("repository_path"),
            Some(&"/path/to/repo".to_string())
        );
    }

    #[test]
    fn test_index_config_default() {
        let config = IndexConfig::default();
        assert_eq!(config.timeout_secs, 300);
        assert_eq!(config.batch_size, 1000);
        assert!(config.branch.is_none());
        assert!(config.commit_hash.is_none());
        assert!(config.skip_if_current);
    }

    #[test]
    fn test_index_report_default() {
        let report = IndexReport::default();
        assert_eq!(report.scanned_files, 0);
        assert_eq!(report.indexed_files, 0);
        assert!(report.branch.is_none());
        assert!(report.skipped_reason.is_none());
    }

    #[test]
    fn test_file_hash() {
        let hash1 = file_hash("test content");
        let hash2 = file_hash("test content");
        let hash3 = file_hash("different content");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn test_normalize_root_file() {
        // Create a temp file to test is_file() behavior
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let normalized = normalize_root(&file_path);
        assert_eq!(normalized, temp_dir.path());
    }

    #[test]
    fn test_normalize_root_dir() {
        let path = PathBuf::from("/some/path");
        let normalized = normalize_root(&path);
        assert_eq!(normalized, PathBuf::from("/some/path"));
    }

    #[test]
    fn test_directory_chain() {
        let root = PathBuf::from("/repo");
        let file_path = PathBuf::from("/repo/src/lib/mod.rs");
        let chain = directory_chain(&root, &file_path);

        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0], PathBuf::from("/repo/src"));
        assert_eq!(chain[1], PathBuf::from("/repo/src/lib"));
    }

    #[test]
    fn test_directory_chain_root_file() {
        let root = PathBuf::from("/repo");
        let file_path = PathBuf::from("/repo/main.rs");
        let chain = directory_chain(&root, &file_path);
        assert!(chain.is_empty());
    }
}
