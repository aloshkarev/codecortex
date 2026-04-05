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
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
        let index_path = path.as_ref().to_path_buf();
        self.index_path_with_branch_context(
            &index_path,
            branch,
            commit_hash,
            &index_path,
            false,
            true,
        )
        .await
    }

    /// Index a path with explicit branch and repository context
    pub async fn index_path_with_branch_context<P: AsRef<Path>, R: AsRef<Path>>(
        &self,
        path: P,
        branch: &str,
        commit_hash: &str,
        repository_path: R,
        force: bool,
        skip_if_current: bool,
    ) -> Result<IndexReport> {
        let mut config = self.config.clone();
        config.branch = Some(branch.to_string());
        config.commit_hash = Some(commit_hash.to_string());
        config.repository_path = Some(repository_path.as_ref().display().to_string());
        config.skip_if_current = skip_if_current;
        self.index_path_with_config(path, force, &config).await
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

        let processed = Arc::new(AtomicUsize::new(0));
        let timed_out = Arc::new(AtomicBool::new(false));

        // ── Phase 1: parse in batches, write nodes immediately ────────────────
        //
        // We parse `PARSE_BATCH` files at a time with rayon, then immediately
        // write the resulting nodes to the DB and drop them.  This keeps peak
        // memory proportional to one batch rather than the entire repository.
        //
        // Edges are accumulated separately because they may reference nodes
        // from other files; we write them all in Phase 2 once every node exists.
        const PARSE_BATCH: usize = 200;

        let repo_id = format!("repo:{}", repository_path);
        let mut seen_dirs: HashSet<String> = HashSet::new();
        let mut all_edges: Vec<CodeEdge> = Vec::new();
        // Lightweight cache-update data: (path, content_hash)
        let mut cache_pairs: Vec<(String, String)> = Vec::new();
        let mut symbol_count = 0usize;
        let mut skipped_files = 0usize;
        let mut indexed_file_count = 0usize;

        for file_batch in files.chunks(PARSE_BATCH) {
            if timed_out.load(Ordering::Relaxed) {
                break;
            }

            // Parse this batch in parallel.
            let parsed: Vec<Result<Option<IndexedFile>>> = {
                let timed_out = Arc::clone(&timed_out);
                let processed = Arc::clone(&processed);
                file_batch
                    .par_iter()
                    .map(|path| {
                        if let Some(timeout) = timeout_duration
                            && start.elapsed() > timeout
                        {
                            timed_out.store(true, Ordering::Relaxed);
                            return Err(CortexError::Timeout("Indexing timed out".to_string()));
                        }
                        let result = self.parse_and_filter_with_config(
                            path,
                            force,
                            &compile_cmd_index,
                            &branch,
                            &repository_path,
                        );
                        processed.fetch_add(1, Ordering::Relaxed);
                        result
                    })
                    .collect()
            };

            // Build nodes for this batch, moving data out of each IndexedFile
            // (no clone) and constructing hierarchy nodes inline.
            let mut batch_nodes: Vec<CodeNode> = Vec::new();

            for item in parsed {
                match item {
                    Ok(Some(mut file)) => {
                        indexed_file_count += 1;
                        symbol_count += file.nodes.len();
                        let file_path = PathBuf::from(&file.path);
                        let file_id = format!("file:{}", file.path);

                        // File hierarchy node
                        batch_nodes.push(CodeNode {
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
                        all_edges.push(CodeEdge {
                            from: repo_id.clone(),
                            to: file_id.clone(),
                            kind: EdgeKind::Contains,
                            properties: HashMap::new(),
                        });

                        // Directory hierarchy (deduped across all batches via seen_dirs)
                        let chain = directory_chain(&root, &file_path);
                        if !chain.is_empty() {
                            for dir in &chain {
                                let dir_key = dir.display().to_string();
                                if seen_dirs.insert(dir_key.clone()) {
                                    batch_nodes.push(CodeNode {
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
                                        properties: build_branch_properties(
                                            &branch,
                                            &repository_path,
                                        ),
                                    });
                                }
                            }

                            if let Some(first_dir) = chain.first() {
                                let first_dir_str = first_dir.display().to_string();
                                all_edges.push(CodeEdge {
                                    from: repo_id.clone(),
                                    to: format!("dir:{first_dir_str}"),
                                    kind: EdgeKind::Contains,
                                    properties: HashMap::new(),
                                });
                            }
                            for window in chain.windows(2) {
                                all_edges.push(CodeEdge {
                                    from: format!("dir:{}", window[0].display()),
                                    to: format!("dir:{}", window[1].display()),
                                    kind: EdgeKind::Contains,
                                    properties: HashMap::new(),
                                });
                            }
                            if let Some(last_dir) = chain.last() {
                                all_edges.push(CodeEdge {
                                    from: format!("dir:{}", last_dir.display()),
                                    to: file_id,
                                    kind: EdgeKind::Contains,
                                    properties: HashMap::new(),
                                });
                            }
                        }

                        // Move symbol nodes out (no clone).  Add branch
                        // properties in-place before appending.
                        let mut sym_nodes = std::mem::take(&mut file.nodes);
                        let branch_props = build_branch_properties(&branch, &repository_path);
                        for node in &mut sym_nodes {
                            node.properties.extend(branch_props.clone());
                        }
                        batch_nodes.extend(sym_nodes);

                        // Move edges out (no clone).
                        all_edges.extend(std::mem::take(&mut file.edges));

                        // Lightweight record for cache update.
                        cache_pairs.push((file.path, file.content_hash));
                        // file (now empty) is dropped here, freeing source strings.
                    }
                    Ok(None) => skipped_files += 1,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse file: {}", e);
                        skipped_files += 1;
                    }
                }
            }

            // Write nodes for this batch immediately, then free the memory.
            self.writer.write_nodes(&batch_nodes).await?;
            // batch_nodes dropped here.
        }

        // ── Phase 2: write edges (all nodes now exist in the DB) ─────────────

        // Collect call-target placeholders from accumulated edges.
        let call_targets: HashSet<(String, String)> = all_edges
            .iter()
            .filter_map(|edge| {
                if !matches!(
                    edge.kind,
                    EdgeKind::Calls | EdgeKind::TypeReference | EdgeKind::FieldAccess
                ) || !edge.to.starts_with("call_target:")
                {
                    return None;
                }
                let name = edge.to.trim_start_matches("call_target:").to_string();
                Some((edge.to.clone(), name))
            })
            .collect();
        let call_targets_vec: Vec<(String, String)> = call_targets.into_iter().collect();
        self.client
            .bulk_upsert_call_targets(&call_targets_vec)
            .await?;

        self.writer.write_edges(&all_edges).await?;
        drop(all_edges);

        let resolved_calls = self
            .client
            .resolve_call_targets(&repository_path, branch.as_deref())
            .await
            .unwrap_or(0);
        let _resolved_type_refs = self
            .client
            .resolve_type_references(&repository_path, branch.as_deref())
            .await
            .unwrap_or(0);
        let _resolved_field_accesses = self
            .client
            .resolve_field_accesses(&repository_path, branch.as_deref())
            .await
            .unwrap_or(0);

        // Build system info for report
        let build_systems: Vec<String> = project_config
            .build_systems
            .iter()
            .map(|b| b.to_string())
            .collect();

        let timed_out = timed_out.load(Ordering::Relaxed);

        // Only promote to "current" if the run completed without timeout.
        // A timed-out run is partial; recording it as current would cause
        // future runs to skip re-indexing, leaving the graph stale.
        if !timed_out {
            if let (Some(br), Some(ch)) = (&branch, &commit_hash) {
                let record = BranchIndexRecord::new(
                    &repository_path,
                    br,
                    ch,
                    indexed_file_count,
                    symbol_count,
                    start.elapsed().as_millis() as u64,
                );
                create_branch_index(&self.client, &record).await?;
            }

            write_cache_entry_pairs(&self.cache, &cache_pairs, &repository_path, &branch)?;
        } else {
            warn!(
                "Indexing timed out — skipping branch index and cache promotion for {}",
                repository_path
            );
        }

        Ok(IndexReport {
            scanned_files,
            indexed_files: indexed_file_count,
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

    #[allow(dead_code)]
    fn update_cache_entries(
        &self,
        files: &[IndexedFile],
        repository_path: &str,
        branch: &Option<String>,
    ) -> Result<()> {
        write_cache_entries(&self.cache, files, repository_path, branch)
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
        branch: &Option<String>,
        repository_path: &str,
    ) -> Result<Option<IndexedFile>> {
        let source = std::fs::read_to_string(path).map_err(|e| CortexError::Io(e.to_string()))?;
        let hash = file_hash(&source);
        let cache_key = cache_key_for_path(path, repository_path, branch);
        if !force
            && self
                .cache
                .get(cache_key.as_bytes())
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

fn cache_key_for_path(path: &Path, repository_path: &str, branch: &Option<String>) -> String {
    let path_key = path.to_string_lossy();
    if let Some(branch) = branch {
        format!("{repository_path}::{branch}::{path_key}")
    } else {
        format!("{repository_path}::{path_key}")
    }
}

/// Write cache entries from lightweight (path, content_hash) pairs.
///
/// This is the primary cache-update path used by the streaming indexer to
/// avoid holding the full `IndexedFile` vector in memory.
fn write_cache_entry_pairs(
    cache: &sled::Db,
    pairs: &[(String, String)],
    repository_path: &str,
    branch: &Option<String>,
) -> Result<()> {
    for (path, content_hash) in pairs {
        let cache_key = cache_key_for_path(Path::new(path), repository_path, branch);
        cache
            .insert(cache_key.as_bytes(), content_hash.as_bytes())
            .map_err(|e| CortexError::Io(e.to_string()))?;
    }
    cache.flush().map_err(|e| CortexError::Io(e.to_string()))?;
    Ok(())
}

/// Write cache entries from full IndexedFile objects (used in tests).
fn write_cache_entries(
    cache: &sled::Db,
    files: &[IndexedFile],
    repository_path: &str,
    branch: &Option<String>,
) -> Result<()> {
    let pairs: Vec<(String, String)> = files
        .iter()
        .map(|f| (f.path.clone(), f.content_hash.clone()))
        .collect();
    write_cache_entry_pairs(cache, &pairs, repository_path, branch)
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
    let include_filter: Option<HashSet<PathBuf>> = std::env::var("CORTEX_INDEX_INCLUDE_FILES")
        .ok()
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(PathBuf::from)
                .collect()
        })
        .filter(|set: &HashSet<PathBuf>| !set.is_empty());
    let mut effective_excludes = config.exclude_patterns.clone();
    if let Ok(raw) = std::env::var("CORTEX_INDEX_EXCLUDE_PATTERNS") {
        for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
            effective_excludes.push(line.to_string());
        }
    }

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

        if let Some(filter) = include_filter.as_ref() {
            let canonical = entry_path
                .canonicalize()
                .unwrap_or_else(|_| entry_path.to_path_buf());
            let absolute = entry_path.to_path_buf();
            if !filter.contains(&canonical) && !filter.contains(&absolute) {
                continue;
            }
        }

        let should_exclude = effective_excludes.iter().any(|pattern| {
            if pattern.ends_with("/**") {
                let dir = &pattern[..pattern.len() - 3];
                if dir.contains('/') || dir.contains('\\') {
                    // Multi-segment pattern like "src/generated/**":
                    // check if the path contains the directory as a substring
                    // bounded by path separators.
                    let dir_with_sep = format!("{}/", dir.replace('\\', "/"));
                    let normalized = path_str.replace('\\', "/");
                    normalized.contains(&dir_with_sep)
                        || normalized.ends_with(&dir_with_sep[..dir_with_sep.len() - 1])
                } else {
                    // Single-segment pattern like "target/**":
                    // match any path component.
                    entry_path
                        .components()
                        .any(|c| c.as_os_str() == std::ffi::OsStr::new(dir))
                }
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
    blake3::hash(source.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

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
        assert_eq!(hash1.len(), 64); // BLAKE3 produces 64 hex chars
    }

    #[test]
    fn test_cache_key_for_path_branch_scoped() {
        let path = Path::new("/repo/src/main.rs");
        let key_main = cache_key_for_path(path, "/repo", &Some("main".to_string()));
        let key_dev = cache_key_for_path(path, "/repo", &Some("dev".to_string()));
        assert_ne!(key_main, key_dev);
    }

    #[test]
    fn test_cache_key_for_path_without_branch() {
        let path = Path::new("/repo/src/main.rs");
        let key = cache_key_for_path(path, "/repo", &None);
        assert_eq!(key, "/repo::/repo/src/main.rs");
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

    #[test]
    fn test_collect_source_files_with_include_filter_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("a.rs");
        let b = temp.path().join("b.rs");
        std::fs::write(&a, "fn a() {}").unwrap();
        std::fs::write(&b, "fn b() {}").unwrap();
        unsafe {
            std::env::remove_var("CORTEX_INDEX_EXCLUDE_PATTERNS");
            std::env::set_var("CORTEX_INDEX_INCLUDE_FILES", a.display().to_string());
        }
        let cfg = ProjectConfig::default();
        let files = collect_source_files_with_config(temp.path(), &cfg);
        unsafe {
            std::env::remove_var("CORTEX_INDEX_INCLUDE_FILES");
        }

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], a);
    }

    #[test]
    fn test_collect_source_files_with_exclude_filter_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("a.rs");
        let b = temp.path().join("b.rs");
        std::fs::write(&a, "fn a() {}").unwrap();
        std::fs::write(&b, "fn b() {}").unwrap();
        unsafe {
            std::env::remove_var("CORTEX_INDEX_INCLUDE_FILES");
            std::env::set_var("CORTEX_INDEX_EXCLUDE_PATTERNS", "b.rs");
        }
        let cfg = ProjectConfig::default();
        let files = collect_source_files_with_config(temp.path(), &cfg);
        unsafe {
            std::env::remove_var("CORTEX_INDEX_EXCLUDE_PATTERNS");
        }

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], a);
    }

    #[test]
    fn test_multi_segment_exclude_pattern() {
        let _lock = ENV_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let gen_dir = temp.path().join("src").join("generated");
        std::fs::create_dir_all(&gen_dir).unwrap();
        let keep = temp.path().join("src").join("main.rs");
        let exclude = gen_dir.join("auto.rs");
        std::fs::write(&keep, "fn keep() {}").unwrap();
        std::fs::write(&exclude, "fn gen() {}").unwrap();
        unsafe {
            std::env::remove_var("CORTEX_INDEX_INCLUDE_FILES");
            std::env::remove_var("CORTEX_INDEX_EXCLUDE_PATTERNS");
        }
        let cfg = ProjectConfig {
            exclude_patterns: vec!["src/generated/**".to_string()],
            ..Default::default()
        };
        let files = collect_source_files_with_config(temp.path(), &cfg);
        assert!(
            files.iter().any(|f| f.ends_with("main.rs")),
            "main.rs should be included"
        );
        assert!(
            !files.iter().any(|f| f.ends_with("auto.rs")),
            "src/generated/auto.rs should be excluded by multi-segment pattern"
        );
    }

    #[test]
    fn test_update_cache_entries_writes_branch_scoped_hashes() {
        let cache = sled::Config::new().temporary(true).open().unwrap();
        let files = vec![IndexedFile {
            path: "/repo/src/main.rs".to_string(),
            language: cortex_core::Language::Rust,
            content_hash: "abc123".to_string(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }];

        write_cache_entries(&cache, &files, "/repo", &Some("main".to_string())).unwrap();

        let value = cache
            .get("/repo::main::/repo/src/main.rs")
            .unwrap()
            .unwrap();
        assert_eq!(value.as_ref(), b"abc123");
    }
}
