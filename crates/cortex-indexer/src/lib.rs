//! # CodeCortex Indexer Library
//!
//! Source code indexing and skeleton generation for efficient code retrieval.
//!
//! ## Overview
//!
//! This crate provides indexing functionality:
//!
//! - **Indexer**: [`Indexer`] for parsing and storing code in the graph database
//! - **Skeleton Builder**: [`SkeletonBuilder`] for compressed code views
//! - **Build Detection**: [`BuildDetector`] for identifying project build systems
//! - **File Hashing**: [`file_hash`] for incremental indexing support
//! - **Timeout Support**: [`IndexConfig`] for configuring indexing behavior
//! - **Progress Tracking**: [`IndexProgress`] for real-time status updates
//! - **Parallel Processing**: Rayon file-parse batches (`indexer_parse_threads`, `indexer_parse_batch_size`)
//! - **Incremental Indexing**: [`IncrementalIndexer`] for efficient re-indexing
//! - **Git-aware Indexing**: [`GitAwareIncremental`] for revision-based tracking
//!
//! ## Supported Build Systems
//!
//! - Cargo (Rust)
//! - npm/pnpm/yarn (JavaScript/TypeScript)
//! - pip/poetry/pipenv (Python)
//! - Go modules
//! - CMake/CompileCommands (C/C++)
//!
//! ## Example
//!
//! ```rust,no_run
//! use cortex_indexer::{Indexer, IndexConfig};
//! use cortex_graph::GraphClient;
//! use std::time::Duration;
//!
//! # async fn example(client: GraphClient) -> Result<(), Box<dyn std::error::Error>> {
//! // Create indexer with default config
//! let indexer = Indexer::new(client.clone(), 1000)?;
//!
//! // Or with custom configuration
//! let config = IndexConfig {
//!     timeout_secs: 60,
//!     batch_size: 500,
//!     max_files: 10000,
//!     progress_callback: None,
//!     ..Default::default()
//! };
//! let indexer = Indexer::with_config(client, config)?;
//!
//! let report = indexer.index_path("/path/to/repo").await?;
//! println!("Indexed {} files in {:.2}s", report.indexed_files, report.duration_secs);
//! # Ok(())
//! # }
//! ```
//!
//! ## Skeleton Generation
//!
//! Skeletons provide compressed code views for LLM context efficiency:
//!
//! ```rust
//! use cortex_indexer::build_skeleton;
//!
//! let code = r#"
//! fn main() {
//!     println!("Hello");
//! }
//! "#;
//! let skeleton = build_skeleton(code, "minimal");
//! // skeleton is a compressed version with signatures only
//! ```
//!
//! ## Incremental Indexing
//!
//! ```rust
//! use cortex_indexer::IncrementalIndexer;
//! use std::path::Path;
//!
//! let mut indexer = IncrementalIndexer::new();
//! indexer.set_revision("abc123");
//! let content = "fn main() {}";
//! let new_content = "fn main() { println!(\"hi\"); }";
//!
//! // Only re-index changed files
//! if indexer.has_file_changed(Path::new("src/main.rs"), content) {
//!     // Re-index the file
//!     indexer.record_file(Path::new("src/main.rs"), new_content);
//! }
//! ```
//!
//! ## Large repositories (kernel-scale trees)
//!
//! For very large trees, keep memory and graph scope under control:
//!
//! - Set **`indexer_max_files`** in [`cortex_core::CortexConfig`] (TOML) to cap how many files one run indexes.
//! - Use **`index_include_files`** in config to index an explicit subset (for example one subsystem) instead of the full tree.
//! - Tune **`index_exclude_patterns`** in config and project excludes so discovery skips generated or vendor trees.
//! - **`falkordb_write_pool_size`** in [`cortex_core::CortexConfig`]: when set to `N > 1`, bulk node upserts and edge UNWIND batches shard across `N` FalkorDB `GRAPH.QUERY` connections (edges shard by `from` id).
//! - **`falkordb_bulk_index_include_source`** (default `false`): omit `source` / `docstring` / JSON `properties` from FalkorDB bulk node UNWIND for faster indexing; set `true` when graph nodes must carry full source during index.
//!
//! ## Multi-process / “distributed” indexing (no cross-machine graph merge)
//!
//! There is no single-process merge of partial symbol tables from different machines. For large trees, run **independent** `cortex index` processes where each process has a **disjoint** file set and a **disjoint graph scope**:
//!
//! - Prefer **`index_include_files`** in config or repeated **`--include-file`** so each job only discovers its shard.
//! - Give each shard a stable, unique graph key via **`--graph-repository-path`** (git-aware CLI) or a distinct working tree root, matching the branch-delete caveats documented above.
//! - Use **separate sled cache files** per shard if needed (`hash_cache_path` / per-home `~/.cortex/hashes.db` isolation), or accept shared cache keys only when `repository_path` strings differ per shard.
//! - **`indexer_parse_pipeline_depth`** > 0 overlaps parsing of the next batch with graph writes for the current batch (bounded to one in-flight batch). **`indexer_parse_threads`**: unset uses host parallelism minus one (see [`crate::default_indexer_parse_threads`]); `0` uses the global Rayon pool; a positive value sets the pool size explicitly.

pub mod build_detector;
mod clones;
mod edge_spill;
pub mod incremental;
mod indexer;
pub mod parallel;
pub mod reach;
pub mod report_analysis;
pub mod skeleton;

pub use clones::{
    CloneAccumulator, compute_clone_pairs, write_clone_edges_to_graph,
};
pub use build_detector::{
    BuildDetector, BuildSystem, CompileCommand, Dependency, DependencyType, ProjectConfig,
};
pub use incremental::{
    ChangeStatus, FileIndexState, FileIndexStatus, GitAwareIncremental, HashEntry,
    IncrementalIndexer, IncrementalStats, IndexChangePlan, IndexRunMode, PlannedFileChange,
};
pub use indexer::{
    EdgeSpillRelTypeTiming, IndexConfig, IndexPhase, IndexProgress, IndexReport, Indexer,
    collect_discoverable_source_files, default_indexer_parse_batch_size,
    default_indexer_parse_threads,
};
pub use parallel::{AdaptiveBatcher, ParallelConfig, ParallelStats};
pub use reach::{
    ReachAccumulator, ReachEntry, ReachIndex, apply_reach_properties, compute_reach_index,
    write_reach_to_graph, REACH_D1_COUNT, REACH_D3_IDS, REACH_TRUNCATED,
};
pub use report_analysis::{
    IndexDerivedKpis, IndexHeuristics, IndexReportAnalysis, PhaseRow, analyze_report, derived_kpis,
};
pub use skeleton::{
    PrecomputedSkeleton, SkeletonBuilder, SkeletonCache, build_skeleton, file_hash, file_hash_fast,
};
