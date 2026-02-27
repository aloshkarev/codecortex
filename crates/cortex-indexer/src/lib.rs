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
//! - **Parallel Processing**: [`ParallelProcessor`] for multi-threaded indexing
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
//! let skeleton = build_skeleton(code);
//! // skeleton is a compressed version with signatures only
//! ```
//!
//! ## Parallel Indexing
//!
//! ```rust
//! use cortex_indexer::{ParallelProcessor, ParallelConfig};
//!
//! let config = ParallelConfig {
//!     num_threads: 4,
//!     min_batch_size: 10,
//!     ..Default::default()
//! };
//! let processor = ParallelProcessor::with_config(config);
//! // Use processor.process_parallel() for parallel file processing
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
//!
//! // Only re-index changed files
//! if indexer.has_file_changed(Path::new("src/main.rs"), content) {
//!     // Re-index the file
//!     indexer.record_file(Path::new("src/main.rs"), new_content);
//! }
//! ```

pub mod build_detector;
pub mod incremental;
mod indexer;
pub mod parallel;
pub mod skeleton;

pub use build_detector::{
    BuildDetector, BuildSystem, CompileCommand, Dependency, DependencyType, ProjectConfig,
};
pub use incremental::{
    ChangeStatus, GitAwareIncremental, HashEntry, IncrementalIndexer, IncrementalStats,
};
pub use indexer::{IndexConfig, IndexPhase, IndexProgress, IndexReport, Indexer};
pub use parallel::{
    AdaptiveBatcher, ParallelConfig, ParallelProcessor, ParallelStats,
};
pub use skeleton::{
    PrecomputedSkeleton, SkeletonBuilder, SkeletonCache, build_skeleton, file_hash,
};
