use crate::build_detector::{BuildDetector, ProjectConfig};
use crate::incremental::{IndexChangePlan, IndexRunMode};
use cortex_core::{
    CodeEdge, CodeNode, CortexError, CortexIgnoreOptions, CortexIgnoreWalker, EdgeKind, EntityKind,
    IndexedFile, Repository, Result, default_global_cortexignore_path,
    ensure_cortexignore_template,
};
use cortex_graph::{
    BranchIndexRecord, GraphClient, NodeWriter, create_branch_index, delete_branch_index,
    is_branch_index_current, upsert_file_tombstone,
};
use cortex_parser::ParserRegistry;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex, Weak};
use std::time::{Duration, Instant};
use tracing::{Level, debug, info, instrument, span, warn};

use crate::edge_spill::{DeferredIndexedSpill, EdgeSpill};

/// Process-global sled hash-cache handles keyed by canonical path.
static HASH_CACHE_REGISTRY: LazyLock<Mutex<HashMap<PathBuf, Weak<sled::Db>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const HASH_CACHE_OPEN_ATTEMPTS: usize = 5;
const HASH_CACHE_RETRY_BASE_MS: u64 = 25;

fn sled_open_error_retryable(error: &sled::Error) -> bool {
    match error {
        sled::Error::Io(io_err) => {
            io_err.kind() == ErrorKind::WouldBlock
                || sled_open_error_text_retryable(io_err.to_string())
        }
        other => sled_open_error_text_retryable(&other.to_string()),
    }
}

fn sled_open_error_text_retryable(error_text: impl AsRef<str>) -> bool {
    let text = error_text.as_ref().to_ascii_lowercase();
    [
        "could not acquire lock",
        "temporarily unavailable",
        "resource temporarily unavailable",
        "wouldblock",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn hash_cache_retry_backoff(attempt: usize) -> Duration {
    let exp = attempt.saturating_sub(1).min(4);
    let base_ms = HASH_CACHE_RETRY_BASE_MS.saturating_mul(1_u64 << exp);
    let jitter_ms = (attempt as u64).wrapping_mul(17) % 40;
    Duration::from_millis(base_ms.saturating_add(jitter_ms).max(1))
}

fn hash_cache_open_error(path: &Path, attempts: usize, error: sled::Error) -> CortexError {
    let detail = error.to_string();
    if sled_open_error_retryable(&error) {
        CortexError::Io(format!(
            "hash cache {path:?} is locked or temporarily unavailable after {attempts} open \
             attempts ({detail}); another process (for example a watch or daemon session) \
             likely holds the sled exclusive lock — stop it or set a distinct hash_cache_path \
             per pipeline (CortexConfig::hash_cache_path / IndexConfig::hash_cache_path); use \
             isolated paths in parallel CI jobs"
        ))
    } else {
        CortexError::Io(format!("open hash cache {path:?}: {detail}"))
    }
}

/// Returns true when this process already holds a live shared sled handle for `path`.
pub fn hash_cache_held_in_process(path: &Path) -> bool {
    let Ok(canon) = canonical_hash_cache_path(path) else {
        return false;
    };
    let Ok(registry) = HASH_CACHE_REGISTRY.lock() else {
        return false;
    };
    registry.get(&canon).and_then(Weak::upgrade).is_some()
}

fn canonical_hash_cache_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        path.canonicalize()
            .map_err(|e| CortexError::Io(format!("canonicalize hash cache path {path:?}: {e}")))
    } else if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            return Ok(path.to_path_buf());
        }
        if parent.exists() {
            let parent_canon = parent.canonicalize().map_err(|e| {
                CortexError::Io(format!("canonicalize hash cache parent {parent:?}: {e}"))
            })?;
            let file_name = path.file_name().map(PathBuf::from).unwrap_or_default();
            Ok(parent_canon.join(file_name))
        } else {
            std::fs::create_dir_all(parent).map_err(|e| {
                CortexError::Io(format!("create hash cache directory {parent:?}: {e}"))
            })?;
            Ok(path.to_path_buf())
        }
    } else {
        Ok(path.to_path_buf())
    }
}

fn open_hash_cache_with_retry(path: &Path) -> std::result::Result<sled::Db, sled::Error> {
    let mut last_err = None;
    for attempt in 1..=HASH_CACHE_OPEN_ATTEMPTS {
        match sled::open(path) {
            Ok(db) => return Ok(db),
            Err(err) if sled_open_error_retryable(&err) && attempt < HASH_CACHE_OPEN_ATTEMPTS => {
                last_err = Some(err);
                std::thread::sleep(hash_cache_retry_backoff(attempt));
            }
            Err(err) => return Err(err),
        }
    }
    Err(last_err.expect("hash cache retry loop ended without capturing last error"))
}

fn acquire_shared_hash_cache(path: PathBuf) -> Result<Arc<sled::Db>> {
    let canon = canonical_hash_cache_path(&path)?;
    if let Some(parent) = canon.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CortexError::Io(format!("create hash cache directory {parent:?}: {e}")))?;
    }

    {
        let registry = HASH_CACHE_REGISTRY
            .lock()
            .map_err(|e| CortexError::Io(format!("hash cache registry poisoned: {e}")))?;
        if let Some(existing) = registry.get(&canon).and_then(Weak::upgrade) {
            return Ok(existing);
        }
    }

    let db = open_hash_cache_with_retry(&canon)
        .map_err(|e| hash_cache_open_error(&canon, HASH_CACHE_OPEN_ATTEMPTS, e))?;
    let arc = Arc::new(db);

    let mut registry = HASH_CACHE_REGISTRY
        .lock()
        .map_err(|e| CortexError::Io(format!("hash cache registry poisoned: {e}")))?;
    registry.retain(|_, weak| weak.strong_count() > 0);
    if let Some(existing) = registry.get(&canon).and_then(Weak::upgrade) {
        return Ok(existing);
    }
    registry.insert(canon, Arc::downgrade(&arc));
    Ok(arc)
}

/// Default Rayon thread count when [`IndexConfig::indexer_parse_threads`] is `None`.
///
/// High-speed defaults use `Some(0)` (global Rayon pool). This fallback uses all host CPUs.
pub fn default_indexer_parse_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().max(1))
        .unwrap_or(1)
}

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
    /// Cap rows per FalkorDB UNWIND write (None = no extra cap beyond `batch_size`).
    pub falkordb_unwind_batch_max: Option<usize>,
    /// Truncate `CodeNode.source` to this many UTF-8 characters (None = keep full source).
    pub graph_node_source_max_bytes: Option<usize>,
    /// Path for indexer hash cache (`sled`). None = [`Indexer::default_hash_cache_path`].
    pub hash_cache_path: Option<PathBuf>,
    /// When non-empty, discovery indexes only these files (repo-root-relative or absolute).
    pub include_files: Vec<PathBuf>,
    /// Extra exclude patterns merged with [`ProjectConfig::exclude_patterns`].
    pub extra_exclude_patterns: Vec<String>,
    /// Optional global `.cortexignore` path (defaults to `~/.cortex/cortexignore` when unset).
    pub global_cortexignore_path: Option<PathBuf>,
    /// Rayon pool size for parse-only work. `None` uses [`default_indexer_parse_threads`]
    /// (host parallelism minus one, at least 1). `Some(0)` uses the global Rayon pool instead.
    pub indexer_parse_threads: Option<usize>,
    /// When > 0, start parsing batch *N+1* while graph work runs for batch *N* (bounded to one in-flight batch).
    pub indexer_parse_pipeline_depth: usize,
    /// Files per Rayon parse batch (conservative default [`default_indexer_parse_batch_size`]).
    /// See [`CortexConfig::indexer_parse_batch_size`].
    pub indexer_parse_batch_size: usize,
    /// Parallel FalkorDB node chunk writes during deferred replay.
    pub falkordb_write_pool_size: usize,
    /// Delete branch graph before parse; skip deferred node spill (see [`CortexConfig::index_force_delete_branch_before_parse`]).
    pub wipe_branch_first: bool,
    /// Precompute incoming-caller reach during indexing (`0` = disabled).
    pub index_reach_depth: usize,
    /// Cap stored caller ids per symbol in the reach index.
    pub index_reach_max_ids: usize,
    /// Index-time MinHash+LSH clone detection (`SIMILAR_TO` edges).
    pub clone_detection_enabled: bool,
}

impl Default for IndexConfig {
    fn default() -> Self {
        let s = cortex_core::indexing_settings(cortex_core::IndexingProfile::Highspeed);
        Self {
            timeout_secs: 7200,
            batch_size: s.max_batch_size,
            max_files: 0,
            progress_callback: None,
            branch: None,
            commit_hash: None,
            repository_path: None,
            skip_if_current: true,
            falkordb_unwind_batch_max: s.falkordb_unwind_batch_max,
            graph_node_source_max_bytes: s.graph_node_source_max_bytes,
            hash_cache_path: None,
            include_files: Vec::new(),
            extra_exclude_patterns: Vec::new(),
            global_cortexignore_path: None,
            indexer_parse_threads: s.indexer_parse_threads,
            indexer_parse_pipeline_depth: s.indexer_parse_pipeline_depth,
            indexer_parse_batch_size: s.indexer_parse_batch_size,
            falkordb_write_pool_size: s.falkordb_write_pool_size,
            wipe_branch_first: false,
            index_reach_depth: 3,
            index_reach_max_ids: 64,
            clone_detection_enabled: false,
        }
    }
}

pub fn default_indexer_parse_batch_size() -> usize {
    cortex_core::DEFAULT_INDEXER_PARSE_BATCH_SIZE
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
/// Full report for one completed index run, including phase timings for performance work.
///
/// ## Reading the timeline
///
/// These **wall-clock** fields partition the run in order; together with
/// [`Self::phase_unattributed_secs`] they should approximate [`Self::duration_secs`]:
/// [`Self::phase_skip_guard_secs`], [`Self::phase_preflight_secs`], [`Self::phase_parse_loop_wall_secs`],
/// [`Self::phase_incremental_cleanup_secs`] *or* [`Self::phase_branch_delete_secs`],
/// [`Self::phase_deferred_node_write_secs`], [`Self::phase_call_targets_secs`],
/// [`Self::phase_edge_flush_secs`], [`Self::phase_resolve_call_targets_secs`],
/// [`Self::phase_resolve_type_references_secs`], [`Self::phase_resolve_field_accesses_secs`],
/// [`Self::phase_promotion_secs`].
///
/// **Inside the parse loop** (subset of [`Self::phase_parse_loop_wall_secs`], not added to the sum above):
/// [`Self::phase_parse_secs`] (CPU in batch workers only), [`Self::phase_incremental_graph_secs`],
/// [`Self::phase_node_write_secs`] (in-loop `write_nodes` only; deferred replay is
/// [`Self::phase_deferred_node_write_secs`]).
///
/// ## Mapping optimizations
///
/// - Faster parsing / Rayon tuning → [`Self::phase_parse_secs`] and possibly lower
///   [`Self::phase_parse_loop_wall_secs`].
/// - Memgraph edge throughput / batching → [`Self::phase_edge_flush_secs`], [`Self::edges_flushed`],
///   and [`Self::edge_flush_bolt_executions`].
/// - Fewer call edges or cheaper call-target work → [`Self::phase_call_targets_secs`],
///   [`Self::call_targets_upserted`], and downstream resolve times.
/// - Cheaper resolution queries → the three `phase_resolve_*` fields.
/// - Smaller forced rebuilds → [`Self::phase_branch_delete_secs`].
pub struct IndexReport {
    pub scanned_files: usize,
    pub indexed_files: usize,
    pub skipped_files: usize,
    /// Source files skipped by `.cortexignore` / `.gitignore` during discovery.
    #[serde(default)]
    pub ignored_file_count: usize,
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
    /// Execution mode used for this run.
    pub mode: IndexRunMode,
    /// Number of deleted files handled by graph cleanup/tombstones.
    pub deleted_files: usize,
    /// Number of file tombstones written.
    pub tombstoned_files: usize,
    /// Number of unchanged files skipped from cache.
    pub cache_hits: usize,
    /// Number of files parsed due to cache miss/force.
    pub cache_misses: usize,
    /// Cache skips that avoided reading file bytes (metadata matched sled JSON entry).
    pub cache_metadata_hits: usize,
    /// CPU seconds in parse worker batches only ([`parse_file_batch_timed`]); subset of
    /// [`Self::phase_parse_loop_wall_secs`]. Faster parsers / Rayon pool tuning show here first.
    pub phase_parse_secs: f64,
    /// Incremental-mode `delete_file_nodes_batch` / tombstone clears **inside** the parse loop only.
    pub phase_incremental_graph_secs: f64,
    /// `write_nodes` during the **main** parse loop only (not deferred replay).
    pub phase_node_write_secs: f64,
    /// Wall seconds: build detection, `upsert_repository`, file discovery, compile-command map,
    /// spill setup — everything before the file-chunk loop.
    pub phase_preflight_secs: f64,
    /// Wall seconds for the entire file-chunk `for` loop (parsing, defer spill pushes, in-loop
    /// graph deletes, in-loop `write_nodes`). Compare to [`Self::phase_parse_secs`] to see
    /// non-parser work inside the loop.
    pub phase_parse_loop_wall_secs: f64,
    /// Incremental mode after the loop: graph deletes + tombstones for deleted paths.
    pub phase_incremental_cleanup_secs: f64,
    /// Forced branch rebuild: `delete_branch_index` after parse (`force` + branch).
    pub phase_branch_delete_secs: f64,
    /// When `force` + branch defers node writes: replay spill + `write_nodes`.
    pub phase_deferred_node_write_secs: f64,
    /// NDJSON read + deserialize during deferred replay (`CORTEX_INDEX_PROFILE=1` or always tracked).
    #[serde(default)]
    pub deferred_spill_read_secs: f64,
    /// File/Directory node assembly during deferred replay.
    #[serde(default)]
    pub deferred_collect_secs: f64,
    /// FalkorDB/Memgraph `bulk_upsert_nodes` during deferred replay.
    #[serde(default)]
    pub deferred_write_nodes_secs: f64,
    /// Bytes written to the deferred node spill file.
    #[serde(default)]
    pub deferred_spill_bytes: u64,
    /// Bulk `CallTarget` upserts before edges reference them.
    pub phase_call_targets_secs: f64,
    /// Number of call-target rows upserted (same length as the resolve input set).
    pub call_targets_upserted: usize,
    /// Time flushing edges from spill to graph (`stream_to_writer`). Larger `batch_size` /
    /// `falkordb_unwind_batch_max`, `CortexConfig::falkordb_write_pool_size` greater than 1
    /// (parallel edge shards), or fewer edges show here.
    pub phase_edge_flush_secs: f64,
    /// Number of edges written during [`Self::phase_edge_flush_secs`].
    pub edges_flushed: u64,
    /// Count of graph write executions during edge flush (Memgraph Bolt `UNWIND` per rel type;
    /// Grafeo native flush: one per chunk). Also exposed as query executions in reports — not
    /// Compare to [`Self::edges_flushed`] when
    /// tuning batches.
    pub edge_flush_bolt_executions: u64,
    pub phase_resolve_call_targets_secs: f64,
    pub phase_resolve_type_references_secs: f64,
    pub phase_resolve_field_accesses_secs: f64,
    /// Branch index record + sled cache promotion at end of a successful run (or stale record on timeout).
    pub phase_promotion_secs: f64,
    /// `skip_if_current` branch guard query when enabled (usually near zero).
    pub phase_skip_guard_secs: f64,
    /// `duration_secs` minus the sum of the phase fields above (rounding, logging, or gaps).
    pub phase_unattributed_secs: f64,
    /// Whether a configured max-files cap truncated the run.
    pub truncated: bool,
    /// Configured max-files cap, when any.
    pub max_files_cap: Option<usize>,
    /// Graph freshness state after this run.
    pub freshness: cortex_core::IndexFreshness,
    /// NDJSON read + deserialize during edge flush (set when `CORTEX_INDEX_PROFILE=1`).
    #[serde(default)]
    pub edge_spill_read_secs: f64,
    /// Time in `write_edges` / Bolt during edge flush (`CORTEX_INDEX_PROFILE=1`).
    #[serde(default)]
    pub edge_spill_bolt_secs: f64,
    /// Per-relationship-type Bolt time during edge flush (`CORTEX_INDEX_PROFILE=1`).
    #[serde(default)]
    pub edge_spill_bolt_by_rel_type: Vec<EdgeSpillRelTypeTiming>,
    /// FalkorDB `GRAPH.QUERY` stats when `CORTEX_FALKORDB_PROFILE=1` or `CORTEX_INDEX_PROFILE=1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub falkordb_profile: Option<cortex_graph::FalkorDbProfileSnapshot>,
}

/// Per Cypher relationship type timing from edge flush profiling.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeSpillRelTypeTiming {
    pub rel_type: String,
    pub bolt_executions: u64,
    pub seconds: f64,
}

pub(crate) fn index_profile_enabled() -> bool {
    std::env::var("CORTEX_INDEX_PROFILE")
        .ok()
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"))
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
    cache: Arc<sled::Db>,
    config: IndexConfig,
    parse_rayon_pool: Option<Arc<rayon::ThreadPool>>,
}

#[derive(Clone)]
struct ParseBatchContext {
    cache: Arc<sled::Db>,
    parser_registry: ParserRegistry,
    compile_cmd_index: Arc<HashMap<PathBuf, crate::build_detector::CompileCommand>>,
    force: bool,
    branch: Option<String>,
    commit_hash: Option<String>,
    repository_path: String,
    parse_rayon_pool: Option<Arc<rayon::ThreadPool>>,
    metadata_hits: Arc<AtomicUsize>,
    index_start: Instant,
    timeout: Option<Duration>,
    timed_out: Arc<AtomicBool>,
    processed: Arc<AtomicUsize>,
}

impl ParseBatchContext {
    fn parse_one_file(&self, path: &Path) -> Result<Option<IndexedFile>> {
        let result = self.parse_one_file_inner(path);
        self.processed.fetch_add(1, Ordering::Relaxed);
        result
    }

    fn parse_one_file_inner(&self, path: &Path) -> Result<Option<IndexedFile>> {
        if let Some(limit) = self.timeout {
            if self.index_start.elapsed() > limit {
                self.timed_out.store(true, Ordering::Relaxed);
                return Err(CortexError::Timeout("Indexing timed out".to_string()));
            }
        }

        let rev = self.commit_hash.as_deref().unwrap_or_default();
        let cache_key = cache_key_for_path(path, &self.repository_path, &self.branch);

        if !self.force {
            if let Ok(Some(entry)) = self.cache.get(cache_key.as_bytes()) {
                let er = entry.as_ref();
                if let Ok(state) = serde_json::from_slice::<crate::incremental::FileIndexState>(er)
                    && state.unchanged_from_disk_metadata(path, rev)
                {
                    self.metadata_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(None);
                }
            }
        }

        let source = std::fs::read_to_string(path).map_err(|e| CortexError::Io(e.to_string()))?;
        let hash = file_hash(&source);
        if !self.force
            && let Some(entry) = self
                .cache
                .get(cache_key.as_bytes())
                .map_err(|e| CortexError::Io(e.to_string()))?
            && cache_entry_matches(entry.as_ref(), path, &source, &hash, rev)
        {
            return Ok(None);
        }

        let parser = self.parser_registry.parser_for_path(path)?;

        let _defines: HashMap<String, Option<String>> = self
            .compile_cmd_index
            .get(path)
            .map(|cmd| cmd.defines())
            .unwrap_or_default();

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

fn parse_file_batch_timed(
    ctx: Arc<ParseBatchContext>,
    paths: Vec<PathBuf>,
) -> Result<(Duration, Vec<Result<Option<IndexedFile>>>)> {
    let t0 = Instant::now();
    let out: Vec<Result<Option<IndexedFile>>> = {
        let run = || {
            paths
                .par_iter()
                .map(|path| ctx.parse_one_file(path))
                .collect()
        };
        match &ctx.parse_rayon_pool {
            Some(pool) => pool.install(run),
            None => run(),
        }
    };
    Ok((t0.elapsed(), out))
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
        let cache_path = config
            .hash_cache_path
            .clone()
            .unwrap_or_else(Self::default_hash_cache_path);
        let cache = acquire_shared_hash_cache(cache_path)?;
        let eff = effective_writer_batch(&config);
        let parse_rayon_pool = match config.indexer_parse_threads {
            Some(0) => None,
            Some(n) => Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(n)
                    .build()
                    .map_err(|e| CortexError::Io(e.to_string()))
                    .map(Arc::new)?,
            ),
            None => Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(default_indexer_parse_threads())
                    .build()
                    .map_err(|e| CortexError::Io(e.to_string()))
                    .map(Arc::new)?,
            ),
        };
        Ok(Self {
            client: client.clone(),
            writer: NodeWriter::new(client, eff),
            parser_registry: ParserRegistry::new(),
            cache,
            config,
            parse_rayon_pool,
        })
    }

    /// Build an indexer using settings from [`cortex_core::CortexConfig`].
    pub fn from_cortex_config(
        client: GraphClient,
        cortex: &cortex_core::CortexConfig,
    ) -> Result<Self> {
        Self::from_cortex_config_with_scan_extras(client, cortex, &[], &[])
    }

    /// Same as [`Self::from_cortex_config`], merging per-run include paths and exclude patterns
    /// (for example CLI `--include-file` / `--exclude-pattern` or project policy).
    pub fn from_cortex_config_with_scan_extras(
        client: GraphClient,
        cortex: &cortex_core::CortexConfig,
        extra_include_files: &[PathBuf],
        extra_exclude_patterns: &[String],
    ) -> Result<Self> {
        let mut c = IndexConfig::default();
        c.batch_size = cortex.max_batch_size;
        c.timeout_secs = cortex.indexer_timeout_secs;
        c.max_files = cortex.indexer_max_files;
        c.falkordb_unwind_batch_max = cortex.falkordb_unwind_batch_max;
        c.graph_node_source_max_bytes = cortex.graph_node_source_max_bytes;
        c.hash_cache_path = cortex.hash_cache_path.clone();
        c.include_files = cortex
            .index_include_files
            .iter()
            .chain(extra_include_files.iter())
            .cloned()
            .collect();
        c.extra_exclude_patterns = cortex
            .index_exclude_patterns
            .iter()
            .chain(extra_exclude_patterns.iter())
            .cloned()
            .collect();
        c.global_cortexignore_path = cortex.global_cortexignore_path.clone();
        c.indexer_parse_threads = cortex.indexer_parse_threads;
        c.indexer_parse_pipeline_depth = cortex.indexer_parse_pipeline_depth;
        c.indexer_parse_batch_size = cortex.indexer_parse_batch_size.max(1);
        c.falkordb_write_pool_size = cortex.falkordb_write_pool_size.max(1);
        c.wipe_branch_first = cortex.index_force_delete_branch_before_parse;
        c.index_reach_depth = cortex.index_reach_depth;
        c.index_reach_max_ids = cortex.index_reach_max_ids;
        c.clone_detection_enabled = cortex.clone_detection_enabled;
        Self::with_config(client, c)
    }

    /// Default sled hash cache path when [`IndexConfig::hash_cache_path`] is unset.
    pub fn default_hash_cache_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cortex")
            .join("hashes.db")
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

    /// Index a path using an explicit production change plan.
    pub async fn index_path_with_branch_change_plan<P: AsRef<Path>, R: AsRef<Path>>(
        &self,
        path: P,
        branch: &str,
        commit_hash: &str,
        repository_path: R,
        change_plan: IndexChangePlan,
        skip_if_current: bool,
    ) -> Result<IndexReport> {
        let mut config = self.config.clone();
        config.branch = Some(branch.to_string());
        config.commit_hash = Some(commit_hash.to_string());
        config.repository_path = Some(repository_path.as_ref().display().to_string());
        config.skip_if_current = skip_if_current;
        self.index_path_with_config_and_plan(path, false, &config, Some(change_plan))
            .await
    }

    /// Index with timeout support
    pub async fn index_path_with_timeout<P: AsRef<Path>>(
        &self,
        path: P,
        force: bool,
        timeout: Duration,
    ) -> Result<IndexReport> {
        let start = Instant::now();

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
        self.index_path_with_config_and_plan(path, force, config, None)
            .await
    }

    async fn index_path_with_config_and_plan<P: AsRef<Path>>(
        &self,
        path: P,
        force: bool,
        config: &IndexConfig,
        explicit_plan: Option<IndexChangePlan>,
    ) -> Result<IndexReport> {
        let span = span!(Level::INFO, "indexing");
        let _enter = span.enter();
        let start = Instant::now();
        if cortex_graph::falkordb_profile_enabled() {
            self.client.reset_falkordb_profile();
        }
        let root = normalize_root(path.as_ref());
        info!(
            path = %root.display(),
            force = force,
            branch = ?config.branch.as_deref(),
            "Starting index operation"
        );

        let repository_path = config
            .repository_path
            .clone()
            .unwrap_or_else(|| root.display().to_string());

        let branch = config.branch.clone();
        let commit_hash = config.commit_hash.clone();
        let mut phase_skip_guard_secs = 0.0;

        if config.skip_if_current
            && let (Some(br), Some(ch)) = (&branch, &commit_hash)
        {
            let sg = Instant::now();
            if is_branch_index_current(&self.client, &repository_path, br, ch).await? {
                phase_skip_guard_secs = sg.elapsed().as_secs_f64();
                return Ok(IndexReport {
                    scanned_files: 0,
                    indexed_files: 0,
                    skipped_files: 0,
                    ignored_file_count: 0,
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
                    mode: IndexRunMode::Skipped,
                    deleted_files: 0,
                    tombstoned_files: 0,
                    cache_hits: 0,
                    cache_misses: 0,
                    cache_metadata_hits: 0,
                    phase_parse_secs: 0.0,
                    phase_incremental_graph_secs: 0.0,
                    phase_node_write_secs: 0.0,
                    phase_preflight_secs: 0.0,
                    phase_parse_loop_wall_secs: 0.0,
                    phase_incremental_cleanup_secs: 0.0,
                    phase_branch_delete_secs: 0.0,
                    phase_deferred_node_write_secs: 0.0,
                    deferred_spill_read_secs: 0.0,
                    deferred_collect_secs: 0.0,
                    deferred_write_nodes_secs: 0.0,
                    deferred_spill_bytes: 0,
                    phase_call_targets_secs: 0.0,
                    call_targets_upserted: 0,
                    phase_edge_flush_secs: 0.0,
                    edges_flushed: 0,
                    edge_flush_bolt_executions: 0,
                    phase_resolve_call_targets_secs: 0.0,
                    phase_resolve_type_references_secs: 0.0,
                    phase_resolve_field_accesses_secs: 0.0,
                    phase_promotion_secs: 0.0,
                    phase_skip_guard_secs,
                    phase_unattributed_secs: (start.elapsed().as_secs_f64()
                        - phase_skip_guard_secs)
                        .max(0.0),
                    truncated: false,
                    max_files_cap: None,
                    freshness: cortex_core::IndexFreshness::Fresh,
                    edge_spill_read_secs: 0.0,
                    edge_spill_bolt_secs: 0.0,
                    edge_spill_bolt_by_rel_type: Vec::new(),
                    falkordb_profile: None,
                });
            }
            phase_skip_guard_secs = sg.elapsed().as_secs_f64();
        }

        let timeout_duration = if config.timeout_secs > 0 {
            Some(Duration::from_secs(config.timeout_secs))
        } else {
            None
        };

        let preflight_t0 = Instant::now();
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

        let mut change_plan = explicit_plan.unwrap_or_else(|| {
            if force {
                IndexChangePlan::full("forced full index")
            } else if !config.include_files.is_empty() {
                IndexChangePlan::incremental("explicit include file set")
            } else {
                IndexChangePlan::incremental("hash-cache incremental scan")
            }
        });

        let (files, ignored_file_count) = if !change_plan.changed_files.is_empty() {
            normalize_explicit_source_files(
                root.as_path(),
                &project_config,
                &change_plan.changed_files,
                &config.extra_exclude_patterns,
                config.global_cortexignore_path.as_deref(),
            )
        } else {
            collect_source_files_with_config(
                root.as_path(),
                &project_config,
                &config.include_files,
                &config.extra_exclude_patterns,
                config.global_cortexignore_path.as_deref(),
            )
        };
        let scanned_files = files.len();

        let files: Vec<_> = if config.max_files > 0 && files.len() > config.max_files {
            change_plan.truncated = true;
            change_plan.max_files_cap = Some(config.max_files);
            files.into_iter().take(config.max_files).collect()
        } else {
            files
        };

        // Build compile command index for C/C++ files (owned for parse worker threads).
        let compile_cmd_index: Arc<HashMap<PathBuf, crate::build_detector::CompileCommand>> =
            Arc::new(
                project_config
                    .compile_commands
                    .iter()
                    .map(|cmd| (cmd.file.clone(), cmd.clone()))
                    .collect(),
            );

        // Small repos: inline node writes beat spill + deferred replay (PERF-102).
        const INLINE_NODE_WRITE_FILE_THRESHOLD: usize = 96;
        let defer_node_writes = force
            && branch.is_some()
            && !config.wipe_branch_first
            && files.len() > INLINE_NODE_WRITE_FILE_THRESHOLD;
        if defer_node_writes {
            info!(
                target: "cortex_indexer::metrics",
                "defer_node_writes enabled (force + branch): nodes spill to disk and replay after parse; \
                 prefer incremental-diff without --force, or --wipe-branch-first for inline writes"
            );
        } else if force
            && branch.is_some()
            && !config.wipe_branch_first
            && files.len() <= INLINE_NODE_WRITE_FILE_THRESHOLD
        {
            info!(
                target: "cortex_indexer::metrics",
                scanned_files = files.len(),
                threshold = INLINE_NODE_WRITE_FILE_THRESHOLD,
                "inline node writes (small repo; skip deferred spill)"
            );
        }
        if config.wipe_branch_first && force && branch.is_some() {
            info!(
                target: "cortex_indexer::metrics",
                "wipe_branch_first: deleting branch graph before parse (no deferred replay)"
            );
        }
        let processed = Arc::new(AtomicUsize::new(0));
        let timed_out = Arc::new(AtomicBool::new(false));
        let repo_id = format!("repo:{}", repository_path);
        let mut seen_dirs = HashSet::new();
        let mut edge_spill = EdgeSpill::new()?;
        let mut reach_acc = if config.index_reach_depth > 0 {
            Some(crate::reach::ReachAccumulator::default())
        } else {
            None
        };
        let mut clone_acc = if config.clone_detection_enabled {
            Some(crate::clones::CloneAccumulator::default())
        } else {
            None
        };
        let mut defer_spill = if defer_node_writes {
            Some(DeferredIndexedSpill::new()?)
        } else {
            None
        };
        let mut cache_pairs = Vec::<(String, String)>::new();
        let mut indexed_file_count = 0usize;
        let mut skipped_files = 0usize;
        let mut symbol_count = 0usize;
        let mut deleted_files = 0usize;
        let mut tombstoned_files = 0usize;
        let parse_batch_size = config.indexer_parse_batch_size.max(1);
        let source_cap = config.graph_node_source_max_bytes;
        let write_chunk = effective_writer_batch(config);
        let phase_preflight_secs = preflight_t0.elapsed().as_secs_f64();

        info!(
            target: "cortex_indexer::metrics",
            phase = "discover",
            scanned_files = files.len(),
            elapsed_ms = start.elapsed().as_millis() as u64,
            "index phase"
        );
        rss_log("after_discover");

        let mut phase_parse = Duration::ZERO;
        let mut phase_incremental_graph = Duration::ZERO;
        let mut phase_node_write = Duration::ZERO;
        let mut phase_branch_delete_secs = 0.0;
        let mut branch_deleted_before_parse = false;

        if config.wipe_branch_first
            && force
            && let Some(br) = branch.as_deref()
        {
            let branch_del = Instant::now();
            let deleted = delete_branch_index(&self.client, &repository_path, br).await?;
            deleted_files += deleted;
            phase_branch_delete_secs = branch_del.elapsed().as_secs_f64();
            branch_deleted_before_parse = true;
            info!(
                target: "cortex_indexer::metrics",
                phase = "branch_delete",
                when = "before_parse",
                elapsed_ms = (phase_branch_delete_secs * 1000.0) as u64,
                deleted_nodes = deleted,
                "index phase"
            );
        }

        let metadata_hits = Arc::new(AtomicUsize::new(0));
        let parse_ctx = Arc::new(ParseBatchContext {
            cache: self.cache.clone(),
            parser_registry: self.parser_registry.clone(),
            compile_cmd_index: Arc::clone(&compile_cmd_index),
            force,
            branch: branch.clone(),
            commit_hash: commit_hash.clone(),
            repository_path: repository_path.clone(),
            parse_rayon_pool: self.parse_rayon_pool.clone(),
            metadata_hits: Arc::clone(&metadata_hits),
            index_start: start,
            timeout: timeout_duration,
            timed_out: Arc::clone(&timed_out),
            processed: Arc::clone(&processed),
        });
        let files_arc = Arc::new(files);
        let chunk_count = files_arc.len().div_ceil(parse_batch_size);
        let pipeline_on = config.indexer_parse_pipeline_depth > 0;

        let mut lookahead: Option<
            tokio::task::JoinHandle<
                std::result::Result<(Duration, Vec<Result<Option<IndexedFile>>>), String>,
            >,
        > = None;

        let parse_loop_t0 = Instant::now();
        for i in 0..chunk_count {
            if timed_out.load(Ordering::Relaxed) {
                break;
            }

            let batch_start = i * parse_batch_size;
            let batch_end = ((i + 1) * parse_batch_size).min(files_arc.len());
            let batch_paths: Vec<PathBuf> = files_arc[batch_start..batch_end].to_vec();

            let (d_parse, parsed) = if pipeline_on {
                if i == 0 {
                    parse_file_batch_timed(Arc::clone(&parse_ctx), batch_paths)?
                } else {
                    let h = lookahead.take().expect("parse lookahead join");
                    h.await
                        .map_err(|e| CortexError::Io(format!("parse batch join: {e}")))?
                        .map_err(|e| CortexError::Io(e))?
                }
            } else {
                parse_file_batch_timed(Arc::clone(&parse_ctx), batch_paths)?
            };
            phase_parse += d_parse;

            if pipeline_on && i + 1 < chunk_count {
                let ctx2 = Arc::clone(&parse_ctx);
                let next_start = (i + 1) * parse_batch_size;
                let next_end = ((i + 2) * parse_batch_size).min(files_arc.len());
                let next_paths: Vec<PathBuf> = files_arc[next_start..next_end].to_vec();
                lookahead = Some(tokio::task::spawn_blocking(move || {
                    parse_file_batch_timed(ctx2, next_paths).map_err(|e| e.to_string())
                }));
            }

            let incremental_paths: Vec<String> = parsed
                .iter()
                .filter_map(|it| {
                    if let Ok(Some(f)) = it {
                        Some(f.path.clone())
                    } else {
                        None
                    }
                })
                .collect();
            let t_inc = Instant::now();
            if change_plan.mode == IndexRunMode::Incremental && !incremental_paths.is_empty() {
                self.client
                    .delete_file_nodes_batch(
                        &repository_path,
                        branch.as_deref(),
                        &incremental_paths,
                    )
                    .await?;
                self.client
                    .clear_file_tombstones_batch(
                        &repository_path,
                        branch.as_deref(),
                        &incremental_paths,
                    )
                    .await?;
            }
            phase_incremental_graph += t_inc.elapsed();

            let mut batch_nodes = Vec::new();
            for item in parsed {
                match item {
                    Ok(Some(mut file)) => {
                        indexed_file_count += 1;
                        symbol_count += file.nodes.len();
                        if let Some(acc) = clone_acc.as_mut() {
                            acc.push_file(&file);
                        }
                        cache_pairs.push((file.path.clone(), file.content_hash.clone()));
                        if defer_node_writes {
                            spill_file_edges(
                                &root,
                                &repo_id,
                                &mut seen_dirs,
                                &mut edge_spill,
                                reach_acc.as_mut(),
                                &mut file,
                            )?;
                            defer_spill
                                .as_mut()
                                .expect("defer spill when forced branch index")
                                .push(&file)?;
                            continue;
                        }

                        append_indexed_file_graph(
                            &root,
                            &repo_id,
                            &branch,
                            &repository_path,
                            &mut seen_dirs,
                            &mut batch_nodes,
                            &mut edge_spill,
                            reach_acc.as_mut(),
                            source_cap,
                            &mut file,
                        )?;
                    }
                    Ok(None) => skipped_files += 1,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse file: {}", e);
                        skipped_files += 1;
                    }
                }
            }

            let t_write = Instant::now();
            if !batch_nodes.is_empty() {
                self.writer.write_nodes(&batch_nodes).await?;
            }
            phase_node_write += t_write.elapsed();
        }
        let phase_parse_loop_wall_secs = parse_loop_t0.elapsed().as_secs_f64();

        info!(
            target: "cortex_indexer::metrics",
            phase = "parse",
            elapsed_ms = phase_parse.as_millis() as u64,
            parse_pipeline = pipeline_on,
            incremental_graph_ms = phase_incremental_graph.as_millis() as u64,
            node_write_ms = phase_node_write.as_millis() as u64,
            "index phase"
        );
        rss_log("after_parse");

        let timed_out = timed_out.load(Ordering::Relaxed);
        change_plan.cache_hits = skipped_files;
        change_plan.cache_misses = indexed_file_count;

        let mut phase_incremental_cleanup_secs = 0.0;

        if !timed_out && change_plan.mode == IndexRunMode::Incremental {
            let t_ic = Instant::now();
            let del_paths: Vec<String> = change_plan
                .deleted_files
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            if !del_paths.is_empty() {
                deleted_files += self
                    .client
                    .delete_file_nodes_batch(&repository_path, branch.as_deref(), &del_paths)
                    .await?;
                for path_text in &del_paths {
                    upsert_file_tombstone(
                        &self.client,
                        &repository_path,
                        branch.as_deref(),
                        path_text,
                        commit_hash.as_deref(),
                        "deleted",
                    )
                    .await?;
                    tombstoned_files += 1;
                }
            }
            phase_incremental_cleanup_secs = t_ic.elapsed().as_secs_f64();
        } else if force
            && !timed_out
            && !branch_deleted_before_parse
            && let Some(br) = branch.as_deref()
        {
            // A forced branch index is a rebuild, not an append. Delete the previous
            // branch graph only after parsing completes, so a timeout cannot erase a
            // healthy existing index.
            let branch_del = Instant::now();
            let deleted = delete_branch_index(&self.client, &repository_path, br).await?;
            deleted_files += deleted;
            phase_branch_delete_secs = branch_del.elapsed().as_secs_f64();
            info!(
                target: "cortex_indexer::metrics",
                phase = "branch_delete",
                elapsed_ms = (phase_branch_delete_secs * 1000.0) as u64,
                deleted_nodes = deleted,
                "index phase"
            );
            info!(
                repository_path = %repository_path,
                branch = br,
                deleted_nodes = deleted,
                "Deleted previous branch graph before forced reindex"
            );
        }

        let mut phase_deferred_node_write_secs = 0.0;
        let mut deferred_spill_read_secs = 0.0;
        let mut deferred_collect_secs = 0.0;
        let mut deferred_write_nodes_secs = 0.0;
        let mut deferred_spill_bytes = 0u64;
        if defer_node_writes {
            let t_def = Instant::now();
            if let Some(ds) = defer_spill.take() {
                deferred_spill_bytes = ds.spill_bytes();
                let (read_secs, collect_secs, write_secs) = replay_deferred_node_spill(
                    &self.writer,
                    ds,
                    &root,
                    &branch,
                    &repository_path,
                    &mut seen_dirs,
                    source_cap,
                    write_chunk,
                    config.falkordb_write_pool_size,
                )
                .await?;
                deferred_spill_read_secs = read_secs;
                deferred_collect_secs = collect_secs;
                deferred_write_nodes_secs = write_secs;
            }
            phase_deferred_node_write_secs = t_def.elapsed().as_secs_f64();
        }

        rss_log("after_nodes");

        if let Some(acc) = reach_acc.as_ref() {
            let reach_index = crate::reach::compute_reach_index(
                acc,
                config.index_reach_depth,
                config.index_reach_max_ids,
            );
            crate::reach::write_reach_to_graph(&self.client, &reach_index, write_chunk).await?;
            info!(
                target: "cortex_indexer::metrics",
                phase = "reach_index",
                symbols = reach_index.entries.len(),
                depth = config.index_reach_depth,
                "reach index written"
            );
        }

        if let Some(acc) = clone_acc.as_ref() {
            let pairs = crate::clones::compute_clone_pairs(acc);
            let replace_clones =
                force || change_plan.mode != crate::incremental::IndexRunMode::Incremental;
            crate::clones::write_clone_edges_to_graph(
                &self.client,
                &repository_path,
                &pairs,
                write_chunk,
                replace_clones,
            )
            .await?;
            info!(
                target: "cortex_indexer::metrics",
                phase = "clone_index",
                pairs = pairs.len(),
                bodies = acc.bodies().len(),
                "clone edges written"
            );
        }

        let call_targets = edge_spill.take_call_target_pairs();
        let call_targets_upserted = call_targets.len();
        let ct_phase = Instant::now();
        self.client.bulk_upsert_call_targets(&call_targets).await?;
        let phase_call_targets_secs = ct_phase.elapsed().as_secs_f64();
        info!(
            target: "cortex_indexer::metrics",
            phase = "call_targets",
            count = call_targets.len(),
            elapsed_ms = ct_phase.elapsed().as_millis() as u64,
            "index phase"
        );

        let edge_flush_t0 = Instant::now();
        let profile_edges = index_profile_enabled();
        let mut edge_bolt_profile = if profile_edges {
            Some(cortex_graph::EdgeWriteProfile::default())
        } else {
            None
        };
        let (edges_written, edge_flush_bolt_executions, edge_flush_detail) = edge_spill
            .stream_to_writer(&self.writer, write_chunk, edge_bolt_profile.as_mut())
            .await?;
        let phase_edge_flush_secs = edge_flush_t0.elapsed().as_secs_f64();
        let (edge_spill_read_secs, edge_spill_bolt_secs, edge_spill_bolt_by_rel_type) =
            if let Some(detail) = edge_flush_detail {
                let mut by_rel = edge_bolt_profile
                    .map(|p| {
                        let mut rows: Vec<EdgeSpillRelTypeTiming> = p
                            .by_rel
                            .into_iter()
                            .map(|(rel_type, stats)| EdgeSpillRelTypeTiming {
                                rel_type,
                                bolt_executions: stats.bolt_executions,
                                seconds: stats.elapsed.as_secs_f64(),
                            })
                            .collect();
                        rows.sort_by(|a, b| {
                            b.seconds
                                .partial_cmp(&a.seconds)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        rows
                    })
                    .unwrap_or_default();
                if by_rel.is_empty() {
                    by_rel = Vec::new();
                }
                (detail.read_secs, detail.bolt_secs, by_rel)
            } else {
                (0.0, 0.0, Vec::new())
            };
        debug!(
            target: "cortex_indexer::metrics",
            phase = "write_edges",
            edges = edges_written,
            bolt_executions = edge_flush_bolt_executions,
            "edge spill flush complete"
        );
        rss_log("after_edges");

        let touched_ct: Vec<String> = call_targets.iter().map(|(id, _)| id.clone()).collect();
        let r1 = Instant::now();
        let resolved_calls = self
            .client
            .resolve_call_targets(&repository_path, branch.as_deref(), &touched_ct)
            .await
            .unwrap_or(0);
        let phase_resolve_call_targets_secs = r1.elapsed().as_secs_f64();
        info!(
            target: "cortex_indexer::metrics",
            phase = "resolve_call_targets",
            elapsed_ms = r1.elapsed().as_millis() as u64,
            "index phase"
        );
        let type_resolve = async {
            let t = Instant::now();
            let n = self
                .client
                .resolve_type_references(&repository_path, branch.as_deref())
                .await
                .unwrap_or(0);
            (t.elapsed(), n)
        };
        let field_resolve = async {
            let t = Instant::now();
            let n = self
                .client
                .resolve_field_accesses(&repository_path, branch.as_deref())
                .await
                .unwrap_or(0);
            (t.elapsed(), n)
        };
        let ((type_elapsed, _resolved_type_refs), (field_elapsed, _resolved_field_accesses)) =
            tokio::join!(type_resolve, field_resolve);
        let phase_resolve_type_references_secs = type_elapsed.as_secs_f64();
        let phase_resolve_field_accesses_secs = field_elapsed.as_secs_f64();
        info!(
            target: "cortex_indexer::metrics",
            phase = "resolve_type_references",
            elapsed_ms = type_elapsed.as_millis() as u64,
            "index phase"
        );
        info!(
            target: "cortex_indexer::metrics",
            phase = "resolve_field_accesses",
            elapsed_ms = field_elapsed.as_millis() as u64,
            "index phase"
        );
        rss_log("after_resolve");

        let build_systems: Vec<String> = project_config
            .build_systems
            .iter()
            .map(|b| b.to_string())
            .collect();

        // Only promote to "current" if the run completed without timeout.
        // A timed-out run is partial; recording it as current would cause
        // future runs to skip re-indexing, leaving the graph stale.
        let promo_t0 = Instant::now();
        if !timed_out {
            if let (Some(br), Some(ch)) = (&branch, &commit_hash) {
                let mut record = BranchIndexRecord::new(
                    &repository_path,
                    br,
                    ch,
                    indexed_file_count,
                    symbol_count,
                    start.elapsed().as_millis() as u64,
                );
                record.file_hash_watermark = Some(file_hash_watermark_pairs(&cache_pairs));
                record.worktree_hash = Some(worktree_hash_from_plan(&change_plan, ch));
                create_branch_index(&self.client, &record).await?;
            }

            write_cache_entry_pairs(
                self.cache.as_ref(),
                &cache_pairs,
                &repository_path,
                &branch,
                &commit_hash,
            )?;
            write_deleted_cache_entries(
                self.cache.as_ref(),
                &change_plan.deleted_files,
                &repository_path,
                &branch,
                &commit_hash,
            )?;
        } else {
            warn!(
                "Indexing timed out — skipping branch index and cache promotion for {}",
                repository_path
            );
            if let (Some(br), Some(ch)) = (&branch, &commit_hash) {
                let mut record = BranchIndexRecord::new(
                    &repository_path,
                    br,
                    ch,
                    indexed_file_count,
                    symbol_count,
                    start.elapsed().as_millis() as u64,
                );
                record.is_stale = true;
                record.graph_freshness = cortex_core::IndexFreshness::Partial;
                record.last_failed_update_reason =
                    Some("index update timed out before promotion".to_string());
                create_branch_index(&self.client, &record).await?;
            }
        }
        let phase_promotion_secs = promo_t0.elapsed().as_secs_f64();

        let duration_secs = start.elapsed().as_secs_f64();
        let accounted_phases = phase_skip_guard_secs
            + phase_preflight_secs
            + phase_parse_loop_wall_secs
            + phase_incremental_cleanup_secs
            + phase_branch_delete_secs
            + phase_deferred_node_write_secs
            + phase_call_targets_secs
            + phase_edge_flush_secs
            + phase_resolve_call_targets_secs
            + phase_resolve_type_references_secs
            + phase_resolve_field_accesses_secs
            + phase_promotion_secs;
        let phase_unattributed_secs = (duration_secs - accounted_phases).max(0.0);

        let falkordb_profile = if cortex_graph::falkordb_profile_enabled() {
            self.client.falkordb_profile_snapshot(true)
        } else {
            None
        };

        Ok(IndexReport {
            scanned_files,
            indexed_files: indexed_file_count,
            skipped_files,
            ignored_file_count,
            resolved_calls,
            build_systems,
            compile_commands_loaded: project_config.compile_commands.len(),
            include_paths_count: project_config.include_paths.len(),
            duration_secs,
            timed_out,
            branch,
            commit_hash,
            symbol_count,
            skipped_reason: None,
            mode: change_plan.mode,
            deleted_files,
            tombstoned_files,
            cache_hits: change_plan.cache_hits,
            cache_misses: change_plan.cache_misses,
            cache_metadata_hits: metadata_hits.load(Ordering::Relaxed),
            phase_parse_secs: phase_parse.as_secs_f64(),
            phase_incremental_graph_secs: phase_incremental_graph.as_secs_f64(),
            phase_node_write_secs: phase_node_write.as_secs_f64(),
            phase_preflight_secs,
            phase_parse_loop_wall_secs,
            phase_incremental_cleanup_secs,
            phase_branch_delete_secs,
            phase_deferred_node_write_secs,
            deferred_spill_read_secs,
            deferred_collect_secs,
            deferred_write_nodes_secs,
            deferred_spill_bytes,
            phase_call_targets_secs,
            call_targets_upserted,
            phase_edge_flush_secs,
            edges_flushed: edges_written,
            edge_flush_bolt_executions,
            phase_resolve_call_targets_secs,
            phase_resolve_type_references_secs,
            phase_resolve_field_accesses_secs,
            phase_promotion_secs,
            phase_skip_guard_secs,
            phase_unattributed_secs,
            truncated: change_plan.truncated,
            max_files_cap: change_plan.max_files_cap,
            freshness: if timed_out {
                cortex_core::IndexFreshness::Partial
            } else {
                cortex_core::IndexFreshness::Fresh
            },
            edge_spill_read_secs,
            edge_spill_bolt_secs,
            edge_spill_bolt_by_rel_type,
            falkordb_profile,
        })
    }

    #[allow(dead_code)]
    fn update_cache_entries(
        &self,
        files: &[IndexedFile],
        repository_path: &str,
        branch: &Option<String>,
        commit_hash: &Option<String>,
    ) -> Result<()> {
        write_cache_entries(
            self.cache.as_ref(),
            files,
            repository_path,
            branch,
            commit_hash,
        )
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
}

fn cache_key_for_path(path: &Path, repository_path: &str, branch: &Option<String>) -> String {
    let path_key = path.to_string_lossy();
    if let Some(branch) = branch {
        format!("{repository_path}::{branch}::{path_key}")
    } else {
        format!("{repository_path}::{path_key}")
    }
}

fn effective_writer_batch(config: &IndexConfig) -> usize {
    let b = config.batch_size.max(1);
    match config.falkordb_unwind_batch_max {
        Some(m) if m > 0 => b.min(m),
        _ => b,
    }
}

/// Best-effort RSS sample (Linux `/proc/self/statm`); no-op on other platforms.
fn rss_log(label: &'static str) {
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = std::fs::read_to_string("/proc/self/statm") {
            let mut parts = contents.split_whitespace();
            if let (Some(size), Some(rss)) = (parts.next(), parts.next()) {
                if let (Ok(size), Ok(rss)) = (size.parse::<u64>(), rss.parse::<u64>()) {
                    let page = 4096u64;
                    info!(
                        target: "cortex_indexer::metrics",
                        phase = "rss",
                        label,
                        size_kb = size.saturating_mul(page) / 1024,
                        rss_kb = rss.saturating_mul(page) / 1024,
                        "memory sample"
                    );
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = label;
    }
}

fn apply_source_cap(nodes: &mut [CodeNode], max_bytes: Option<usize>) {
    let Some(max) = max_bytes.filter(|&m| m > 0) else {
        return;
    };
    for node in nodes.iter_mut() {
        if let Some(ref mut src) = node.source {
            if src.len() > max {
                let mut end = max;
                while end > 0 && !src.is_char_boundary(end) {
                    end -= 1;
                }
                src.truncate(end);
            }
        }
    }
}

/// Replay deferred spill: returns `(read_secs, collect_secs, write_secs)`.
async fn replay_deferred_node_spill(
    writer: &cortex_graph::NodeWriter,
    spill: crate::edge_spill::DeferredIndexedSpill,
    root: &Path,
    branch: &Option<String>,
    repository_path: &str,
    seen_dirs: &mut HashSet<String>,
    source_cap: Option<usize>,
    write_chunk: usize,
    write_parallel: usize,
) -> Result<(f64, f64, f64)> {
    use std::io::BufRead;

    let reader = spill.into_buffered_reader()?;
    let flush_at = write_chunk.max(1);
    let mut pending: Vec<CodeNode> = Vec::new();
    let mut read_time = Duration::ZERO;
    let mut collect_time = Duration::ZERO;
    let mut write_time = Duration::ZERO;

    for line in reader.lines() {
        let t_read = Instant::now();
        let line = line.map_err(|e| CortexError::Io(e.to_string()))?;
        if line.is_empty() {
            continue;
        }
        let mut record = crate::edge_spill::parse_deferred_spill_line(&line)?;
        read_time += t_read.elapsed();

        let t_collect = Instant::now();
        let mut batch_nodes = Vec::new();
        replay_deferred_file_nodes(
            root,
            branch,
            repository_path,
            seen_dirs,
            &mut batch_nodes,
            source_cap,
            &mut record,
        )?;
        collect_time += t_collect.elapsed();

        pending.append(&mut batch_nodes);
        if pending.len() >= flush_at {
            let t_write = Instant::now();
            writer
                .write_nodes_concurrent(&pending, write_parallel)
                .await?;
            write_time += t_write.elapsed();
            pending.clear();
        }
    }
    if !pending.is_empty() {
        let t_write = Instant::now();
        writer
            .write_nodes_concurrent(&pending, write_parallel)
            .await?;
        write_time += t_write.elapsed();
    }

    Ok((
        read_time.as_secs_f64(),
        collect_time.as_secs_f64(),
        write_time.as_secs_f64(),
    ))
}

/// Fast replay: reuse parsed symbol nodes from spill; only add File/Directory shells.
fn replay_deferred_file_nodes(
    root: &Path,
    branch: &Option<String>,
    repository_path: &str,
    seen_dirs: &mut HashSet<String>,
    nodes: &mut Vec<CodeNode>,
    source_cap: Option<usize>,
    record: &mut crate::edge_spill::DeferredFileRecord,
) -> Result<()> {
    append_file_and_directory_nodes(root, branch, repository_path, seen_dirs, nodes, record)?;
    let branch_props = build_branch_properties(branch, repository_path);
    let mut file_nodes = record.nodes()?;
    for node in &mut file_nodes {
        merge_branch_properties(&mut node.properties, &branch_props);
    }
    apply_source_cap(&mut file_nodes, source_cap);
    nodes.append(&mut file_nodes);
    Ok(())
}

fn append_file_and_directory_nodes(
    root: &Path,
    branch: &Option<String>,
    repository_path: &str,
    seen_dirs: &mut HashSet<String>,
    nodes: &mut Vec<CodeNode>,
    record: &crate::edge_spill::DeferredFileRecord,
) -> Result<()> {
    let file_path = PathBuf::from(&record.path);
    let file_id = format!("file:{}", record.path);

    nodes.push(CodeNode {
        id: file_id,
        kind: EntityKind::File,
        name: file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string(),
        path: Some(record.path.clone()),
        line_number: Some(1),
        lang: Some(record.language),
        source: None,
        docstring: None,
        properties: build_branch_properties(branch, repository_path),
    });

    let chain = directory_chain(root, &file_path);
    for dir in chain {
        let dir_key = dir.display().to_string();
        if seen_dirs.insert(dir_key.clone()) {
            nodes.push(CodeNode {
                id: format!("dir:{dir_key}"),
                kind: EntityKind::Directory,
                name: dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string(),
                path: Some(dir_key),
                line_number: Some(1),
                lang: None,
                source: None,
                docstring: None,
                properties: build_branch_properties(branch, repository_path),
            });
        }
    }
    Ok(())
}

fn append_indexed_file_graph(
    root: &Path,
    repo_id: &str,
    branch: &Option<String>,
    repository_path: &str,
    seen_dirs: &mut HashSet<String>,
    nodes: &mut Vec<CodeNode>,
    edge_spill: &mut EdgeSpill,
    reach_acc: Option<&mut crate::reach::ReachAccumulator>,
    source_cap: Option<usize>,
    file: &mut IndexedFile,
) -> Result<()> {
    spill_file_edges(root, repo_id, seen_dirs, edge_spill, reach_acc, file)?;
    collect_file_nodes(
        root,
        branch,
        repository_path,
        seen_dirs,
        nodes,
        source_cap,
        file,
    )?;
    Ok(())
}

/// Spill structural and symbol edges during parse when node writes are deferred.
fn spill_file_edges(
    root: &Path,
    repo_id: &str,
    seen_dirs: &mut HashSet<String>,
    edge_spill: &mut EdgeSpill,
    mut reach_acc: Option<&mut crate::reach::ReachAccumulator>,
    file: &mut IndexedFile,
) -> Result<()> {
    let file_path = PathBuf::from(&file.path);
    let file_id = format!("file:{}", file.path);

    let chain = directory_chain(root, &file_path);
    let mut parent = repo_id.to_string();
    for dir in chain {
        let dir_key = dir.display().to_string();
        let dir_id = format!("dir:{dir_key}");
        if seen_dirs.insert(dir_key) {
            edge_spill.push(&CodeEdge {
                from: parent,
                to: dir_id.clone(),
                kind: EdgeKind::Contains,
                properties: HashMap::new(),
            })?;
        }
        parent = dir_id;
    }
    edge_spill.push(&CodeEdge {
        from: parent,
        to: file_id,
        kind: EdgeKind::Contains,
        properties: HashMap::new(),
    })?;

    for edge in file.edges.drain(..) {
        if let Some(acc) = reach_acc.as_mut() {
            acc.push_edge(&edge);
        }
        edge_spill.push(&edge)?;
    }
    Ok(())
}

/// Build graph nodes for one file (no edges — use [`spill_file_edges`] when deferring writes).
fn collect_file_nodes(
    root: &Path,
    branch: &Option<String>,
    repository_path: &str,
    seen_dirs: &mut HashSet<String>,
    nodes: &mut Vec<CodeNode>,
    source_cap: Option<usize>,
    file: &mut IndexedFile,
) -> Result<()> {
    let record = crate::edge_spill::DeferredFileRecord::from(&*file);
    append_file_and_directory_nodes(root, branch, repository_path, seen_dirs, nodes, &record)?;

    let branch_props = build_branch_properties(branch, repository_path);
    for node in &mut file.nodes {
        merge_branch_properties(&mut node.properties, &branch_props);
    }
    apply_source_cap(&mut file.nodes, source_cap);
    nodes.append(&mut file.nodes);
    Ok(())
}

#[allow(dead_code)]
fn write_cache_entries(
    cache: &sled::Db,
    files: &[IndexedFile],
    repository_path: &str,
    branch: &Option<String>,
    commit_hash: &Option<String>,
) -> Result<()> {
    for file in files {
        let cache_key = cache_key_for_path(Path::new(&file.path), repository_path, branch);
        let state = crate::incremental::FileIndexState::from_content(
            Path::new(&file.path),
            "",
            repository_path,
            branch.as_deref(),
            commit_hash.as_deref().unwrap_or_default(),
        );
        let mut state = state;
        state.content_hash = file.content_hash.clone();
        state.file_size = std::fs::metadata(&file.path).map(|m| m.len()).unwrap_or(0);
        let value = serde_json::to_vec(&state).map_err(|e| CortexError::Io(e.to_string()))?;
        cache
            .insert(cache_key.as_bytes(), value)
            .map_err(|e| CortexError::Io(e.to_string()))?;
    }
    cache.flush().map_err(|e| CortexError::Io(e.to_string()))?;
    Ok(())
}

fn write_cache_entry_pairs(
    cache: &sled::Db,
    files: &[(String, String)],
    repository_path: &str,
    branch: &Option<String>,
    commit_hash: &Option<String>,
) -> Result<()> {
    for (path, content_hash) in files {
        let cache_key = cache_key_for_path(Path::new(path), repository_path, branch);
        let mut state = crate::incremental::FileIndexState::from_content(
            Path::new(path),
            "",
            repository_path,
            branch.as_deref(),
            commit_hash.as_deref().unwrap_or_default(),
        );
        state.content_hash = content_hash.clone();
        state.file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let value = serde_json::to_vec(&state).map_err(|e| CortexError::Io(e.to_string()))?;
        cache
            .insert(cache_key.as_bytes(), value)
            .map_err(|e| CortexError::Io(e.to_string()))?;
    }
    cache.flush().map_err(|e| CortexError::Io(e.to_string()))?;
    Ok(())
}

fn write_deleted_cache_entries(
    cache: &sled::Db,
    files: &[PathBuf],
    repository_path: &str,
    branch: &Option<String>,
    commit_hash: &Option<String>,
) -> Result<()> {
    for path in files {
        let cache_key = cache_key_for_path(path, repository_path, branch);
        let state = crate::incremental::FileIndexState::tombstone(
            path,
            repository_path,
            branch.as_deref(),
            commit_hash.as_deref().unwrap_or_default(),
        );
        let value = serde_json::to_vec(&state).map_err(|e| CortexError::Io(e.to_string()))?;
        cache
            .insert(cache_key.as_bytes(), value)
            .map_err(|e| CortexError::Io(e.to_string()))?;
    }
    cache.flush().map_err(|e| CortexError::Io(e.to_string()))?;
    Ok(())
}

fn cache_entry_matches(
    raw: &[u8],
    path: &Path,
    source: &str,
    content_hash: &str,
    revision: &str,
) -> bool {
    // Backward compatibility: older cache entries were stored as raw SHA-256 bytes.
    if raw == content_hash.as_bytes() {
        return true;
    }

    serde_json::from_slice::<crate::incremental::FileIndexState>(raw)
        .map(|state| state.matches_content(path, source, revision))
        .unwrap_or(false)
}

fn build_ignore_walker(
    scan_path: &Path,
    project: &ProjectConfig,
    extra_exclude_patterns: &[String],
    global_ignore_path: Option<&Path>,
) -> CortexIgnoreWalker {
    let scan_root = normalize_root(scan_path);
    let repo_root = cortex_core::find_git_repository_root(&scan_root)
        .map(|p| p.canonicalize().unwrap_or(p))
        .unwrap_or_else(|| scan_root.clone());

    let template_root = if repo_root.is_dir() {
        repo_root.clone()
    } else {
        scan_root.clone()
    };
    let default_patterns = cortex_core::ProjectConfig::default().ignore_patterns;
    let _ = ensure_cortexignore_template(&template_root, &default_patterns);

    let mut policy = project.exclude_patterns.clone();
    policy.extend_from_slice(extra_exclude_patterns);

    let global = global_ignore_path
        .map(PathBuf::from)
        .or_else(default_global_cortexignore_path);

    CortexIgnoreWalker::new(CortexIgnoreOptions {
        repo_root,
        scan_root: Some(scan_root),
        global_ignore_path: global,
        respect_gitignore: true,
        respect_cortexignore: true,
        include_hidden: false,
        policy_excludes: policy,
        count_ignored_skips: true,
    })
}

fn normalize_explicit_source_files(
    root: &Path,
    config: &ProjectConfig,
    files: &[PathBuf],
    extra_exclude_patterns: &[String],
    global_ignore_path: Option<&Path>,
) -> (Vec<PathBuf>, usize) {
    let walker = build_ignore_walker(root, config, extra_exclude_patterns, global_ignore_path);
    walker
        .collect_files_with_stats(root, Some(files), |p| {
            cortex_core::Language::from_path(p).is_some()
        })
        .map(|stats| (stats.files, stats.ignored_by_rules))
        .unwrap_or((Vec::new(), 0))
}

fn collect_source_files_with_config(
    path: &Path,
    project: &ProjectConfig,
    include_files: &[PathBuf],
    extra_exclude_patterns: &[String],
    global_ignore_path: Option<&Path>,
) -> (Vec<PathBuf>, usize) {
    let walker = build_ignore_walker(path, project, extra_exclude_patterns, global_ignore_path);
    let include = if include_files.is_empty() {
        None
    } else {
        Some(include_files)
    };
    walker
        .collect_files_with_stats(path, include, |p| {
            cortex_core::Language::from_path(p).is_some()
        })
        .map(|stats| (stats.files, stats.ignored_by_rules))
        .unwrap_or((Vec::new(), 0))
}

/// Discover indexable source files using the same ignore rules as graph indexing.
pub fn collect_discoverable_source_files(
    path: &Path,
    project: &ProjectConfig,
    include_files: &[PathBuf],
    extra_exclude_patterns: &[String],
    global_ignore_path: Option<&Path>,
) -> Vec<PathBuf> {
    collect_source_files_with_config(
        path,
        project,
        include_files,
        extra_exclude_patterns,
        global_ignore_path,
    )
    .0
}

#[allow(dead_code)]
fn file_hash_watermark(files: &[IndexedFile]) -> String {
    let mut pairs: Vec<_> = files
        .iter()
        .map(|file| (file.path.as_str(), file.content_hash.as_str()))
        .collect();
    pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut hasher = Sha256::new();
    for (path, hash) in pairs {
        hasher.update(path.as_bytes());
        hasher.update(b"\0");
        hasher.update(hash.as_bytes());
        hasher.update(b"\0");
    }
    format!("{:x}", hasher.finalize())
}

fn file_hash_watermark_pairs(files: &[(String, String)]) -> String {
    let mut pairs: Vec<_> = files
        .iter()
        .map(|(path, hash)| (path.as_str(), hash.as_str()))
        .collect();
    pairs.sort_unstable_by(|a, b| a.0.cmp(b.0));
    let mut hasher = Sha256::new();
    for (path, hash) in pairs {
        hasher.update(path.as_bytes());
        hasher.update(b"\0");
        hasher.update(hash.as_bytes());
        hasher.update(b"\0");
    }
    format!("{:x}", hasher.finalize())
}

fn worktree_hash_from_plan(plan: &IndexChangePlan, commit_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(commit_hash.as_bytes());
    for path in &plan.changed_files {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(b"\0changed\0");
    }
    for path in &plan.deleted_files {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(b"\0deleted\0");
    }
    format!("{:x}", hasher.finalize())
}

/// Attach branch/repo metadata without cloning the whole map per node.
fn merge_branch_properties(node: &mut HashMap<String, String>, branch: &HashMap<String, String>) {
    for (k, v) in branch {
        node.insert(k.clone(), v.clone());
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

pub fn file_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::Language;
    use tempfile::TempDir;

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
        let hs = cortex_core::indexing_settings(cortex_core::IndexingProfile::Highspeed);
        let config = IndexConfig::default();
        assert_eq!(config.timeout_secs, 7200);
        assert_eq!(config.batch_size, hs.max_batch_size);
        assert!(config.branch.is_none());
        assert!(config.commit_hash.is_none());
        assert!(config.skip_if_current);
        assert_eq!(
            config.falkordb_unwind_batch_max,
            hs.falkordb_unwind_batch_max
        );
        assert_eq!(
            config.graph_node_source_max_bytes,
            hs.graph_node_source_max_bytes
        );
        assert_eq!(
            config.indexer_parse_pipeline_depth,
            hs.indexer_parse_pipeline_depth
        );
        assert_eq!(config.indexer_parse_batch_size, hs.indexer_parse_batch_size);
    }

    #[test]
    fn default_indexer_parse_threads_respects_host() {
        let n = default_indexer_parse_threads();
        assert!(n >= 1);
        if let Ok(ap) = std::thread::available_parallelism() {
            assert_eq!(n, ap.get().max(1));
        }
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
    fn test_collect_source_files_with_include_filter() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("a.rs");
        let b = temp.path().join("b.rs");
        let nested = temp.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let c = nested.join("c.rs");
        std::fs::write(&a, "fn a() {}").unwrap();
        std::fs::write(&b, "fn b() {}").unwrap();
        std::fs::write(&c, "fn c() {}").unwrap();
        let cfg = ProjectConfig::default();
        let include = vec![PathBuf::from("a.rs"), PathBuf::from("nested/c.rs")];
        let (files, _) = collect_source_files_with_config(temp.path(), &cfg, &include, &[], None);

        let a = a.canonicalize().unwrap();
        let b = b.canonicalize().unwrap();
        let c = c.canonicalize().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&a));
        assert!(files.contains(&c));
        assert!(!files.contains(&b));
    }

    #[test]
    fn test_collect_source_files_include_filter_avoids_full_walk_and_missing_files() {
        let temp = tempfile::tempdir().unwrap();
        let keep = temp.path().join("keep.rs");
        let ignored_dir = temp.path().join("ignored");
        std::fs::create_dir_all(&ignored_dir).unwrap();
        let ignored = ignored_dir.join("ignored.rs");
        let unsupported = temp.path().join("notes.txt");
        std::fs::write(&keep, "fn keep() {}").unwrap();
        std::fs::write(&ignored, "fn ignored() {}").unwrap();
        std::fs::write(&unsupported, "notes").unwrap();
        let cfg = ProjectConfig {
            exclude_patterns: vec!["ignored/**".to_string()],
            ..Default::default()
        };
        let include = vec![
            PathBuf::from("keep.rs"),
            PathBuf::from("ignored/ignored.rs"),
            PathBuf::from("notes.txt"),
            PathBuf::from("missing.rs"),
        ];
        let (files, _) = collect_source_files_with_config(temp.path(), &cfg, &include, &[], None);

        assert_eq!(files, vec![keep.canonicalize().unwrap()]);
    }

    #[test]
    fn test_collect_source_files_with_extra_exclude_patterns() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("a.rs");
        let b = temp.path().join("b.rs");
        std::fs::write(&a, "fn a() {}").unwrap();
        std::fs::write(&b, "fn b() {}").unwrap();
        let cfg = ProjectConfig::default();
        let (files, _) =
            collect_source_files_with_config(temp.path(), &cfg, &[], &["b.rs".to_string()], None);

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], a);
    }

    #[test]
    fn test_collect_source_files_respects_cortexignore() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join(".cortexignore"), "skip.rs\n").unwrap();
        std::fs::write(temp.path().join("keep.rs"), "fn keep() {}").unwrap();
        std::fs::write(temp.path().join("skip.rs"), "fn skip() {}").unwrap();
        let cfg = ProjectConfig::default();
        let (files, _) = collect_source_files_with_config(temp.path(), &cfg, &[], &[], None);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("keep.rs"));
    }

    #[test]
    fn test_multi_segment_exclude_pattern() {
        let temp = tempfile::tempdir().unwrap();
        let gen_dir = temp.path().join("src").join("generated");
        std::fs::create_dir_all(&gen_dir).unwrap();
        let keep = temp.path().join("src").join("main.rs");
        let exclude = gen_dir.join("auto.rs");
        std::fs::write(&keep, "fn keep() {}").unwrap();
        std::fs::write(&exclude, "fn gen() {}").unwrap();
        let cfg = ProjectConfig {
            exclude_patterns: vec!["src/generated/**".to_string()],
            ..Default::default()
        };
        let (files, _) = collect_source_files_with_config(temp.path(), &cfg, &[], &[], None);
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

        write_cache_entries(
            &cache,
            &files,
            "/repo",
            &Some("main".to_string()),
            &Some("rev1".to_string()),
        )
        .unwrap();

        let value = cache
            .get("/repo::main::/repo/src/main.rs")
            .unwrap()
            .unwrap();
        let state: crate::incremental::FileIndexState = serde_json::from_slice(&value).unwrap();
        assert_eq!(state.content_hash, "abc123");
        assert_eq!(state.branch.as_deref(), Some("main"));
        assert_eq!(state.revision, "rev1");
    }

    #[test]
    fn test_cache_entry_matches_unified_state_and_rejects_tombstone() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("src.rs");
        std::fs::write(&file, "fn main() {}").unwrap();
        let state = crate::incremental::FileIndexState::from_content(
            &file,
            "fn main() {}",
            dir.path().to_str().unwrap(),
            Some("main"),
            "rev1",
        );
        let raw = serde_json::to_vec(&state).unwrap();
        assert!(cache_entry_matches(
            &raw,
            &file,
            "fn main() {}",
            &state.content_hash,
            "rev1"
        ));

        let tombstone = crate::incremental::FileIndexState::tombstone(
            &file,
            dir.path().to_str().unwrap(),
            Some("main"),
            "rev1",
        );
        let raw = serde_json::to_vec(&tombstone).unwrap();
        assert!(!cache_entry_matches(
            &raw,
            &file,
            "fn main() {}",
            &state.content_hash,
            "rev1"
        ));
    }

    #[test]
    fn hash_cache_held_in_process_tracks_open_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("hashes.db");
        let unknown = dir.path().join("other.db");

        assert!(!hash_cache_held_in_process(&unknown));
        assert!(!hash_cache_held_in_process(&cache_path));

        let _cache = acquire_shared_hash_cache(cache_path.clone()).unwrap();
        assert!(hash_cache_held_in_process(&cache_path));
        assert!(!hash_cache_held_in_process(&unknown));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn two_indexers_on_same_cache_path_succeed_in_one_process() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("hashes.db");

        let exclusive = sled::open(&cache_path).unwrap();
        assert!(
            sled::open(&cache_path).is_err(),
            "second exclusive sled::open on the same path should fail"
        );
        drop(exclusive);

        let cache_a = acquire_shared_hash_cache(cache_path.clone()).unwrap();
        let cache_b = acquire_shared_hash_cache(cache_path.clone()).unwrap();
        cache_a.insert(b"probe", b"1").unwrap();
        assert_eq!(
            cache_b.get(b"probe").unwrap().as_deref(),
            Some(b"1" as &[u8])
        );

        let config = IndexConfig {
            hash_cache_path: Some(cache_path.clone()),
            indexer_parse_threads: Some(0),
            ..Default::default()
        };
        if let Ok(Ok(client)) = tokio::time::timeout(
            Duration::from_millis(500),
            cortex_graph::GraphClient::connect(&cortex_core::CortexConfig::default()),
        )
        .await
        {
            Indexer::with_config(client.clone(), config.clone())
                .expect("first indexer on shared cache path");
            Indexer::with_config(client, config).expect("second indexer on shared cache path");
        } else {
            let third = acquire_shared_hash_cache(cache_path).unwrap();
            assert_eq!(
                third.get(b"probe").unwrap().as_deref(),
                Some(b"1" as &[u8]),
                "shared hash cache remains usable without a live graph client"
            );
        }
    }

    #[test]
    fn defer_path_spills_edges_once_collect_has_no_symbol_edges() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let repo_id = "repo:test";
        let branch = Some("main".to_string());
        let repository_path = "/test";
        let mut seen_dirs = HashSet::new();
        let mut edge_spill = EdgeSpill::new().unwrap();
        let mut file = IndexedFile {
            path: "src/lib.rs".to_string(),
            language: Language::Rust,
            content_hash: "abc".to_string(),
            nodes: vec![CodeNode {
                id: "sym:1".to_string(),
                kind: EntityKind::Function,
                name: "foo".to_string(),
                path: Some("src/lib.rs".to_string()),
                line_number: Some(1),
                lang: Some(Language::Rust),
                source: None,
                docstring: None,
                properties: HashMap::new(),
            }],
            edges: vec![CodeEdge {
                from: "sym:1".to_string(),
                to: "sym:2".to_string(),
                kind: EdgeKind::Calls,
                properties: HashMap::new(),
            }],
        };
        spill_file_edges(
            root,
            repo_id,
            &mut seen_dirs,
            &mut edge_spill,
            None,
            &mut file,
        )
        .unwrap();
        assert!(file.edges.is_empty());
        // repo→dir…→file (deduped CONTAINS) + 1 symbol edge
        assert!(edge_spill.edge_count() >= 3);

        let mut nodes = Vec::new();
        collect_file_nodes(
            root,
            &branch,
            repository_path,
            &mut seen_dirs,
            &mut nodes,
            None,
            &mut file,
        )
        .unwrap();
        assert!(file.edges.is_empty());
        assert_eq!(edge_spill.edge_count(), 3);
        assert!(!nodes.is_empty());
    }
}
