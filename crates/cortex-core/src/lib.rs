//! # CodeCortex Core Library
//!
//! Core types, error handling, and utilities for the CodeCortex code intelligence system.
//!
//! ## Overview
//!
//! This crate provides the foundational types and utilities used across all CodeCortex crates:
//!
//! - **Code Models**: [`CodeNode`], [`CodeEdge`], [`Repository`] for representing code structure
//! - **Language Support**: [`Language`] enum with detection from file extensions (10 languages)
//! - **Complexity Analysis**: [`compute_cyclomatic_complexity`] for code complexity metrics
//! - **Configuration**: [`CortexConfig`] for application settings
//! - **Error Handling**: [`CortexError`] and [`Result`] types
//! - **Project Management**: [`ProjectState`], [`GitInfo`] for project tracking
//! - **Git Operations**: [`GitOperations`] for history traversal, blame, and branch comparison
//!
//! ## Supported Languages
//!
//! | Language | Extensions |
//! |----------|------------|
//! | Rust | `.rs` |
//! | Python | `.py` |
//! | Go | `.go` |
//! | TypeScript | `.ts`, `.tsx` |
//! | JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` |
//! | C | `.c`, `.h` |
//! | C++ | `.cpp`, `.hpp`, `.cc`, `.cxx` |
//! | Java | `.java` |
//! | PHP | `.php` |
//! | Ruby | `.rb` |
//!
//! ## Example
//!
//! ```rust
//! use cortex_core::{CodeNode, EntityKind, Language, compute_cyclomatic_complexity};
//!
//! // Create a code node
//! let node = CodeNode {
//!     id: "func:main".to_string(),
//!     kind: EntityKind::Function,
//!     name: "main".to_string(),
//!     path: Some("src/main.rs".to_string()),
//!     line_number: Some(1),
//!     lang: Some(Language::Rust),
//!     source: Some("fn main() {}".to_string()),
//!     docstring: None,
//!     properties: Default::default(),
//! };
//!
//! // Compute complexity
//! let complexity = compute_cyclomatic_complexity("fn test() { if x { a } else { b } }");
//! assert_eq!(complexity, 2);
//! ```
//!
//! ## Feature Flags
//!
//! This crate has no feature flags - all functionality is always available.

pub mod a2a_config;
pub mod complexity;
pub mod config;
pub mod error;
pub mod git;
pub mod ignore;
pub mod index_scope;
pub mod language;
pub mod mcp_config;
pub mod model;
pub mod project;
pub mod tokens;

pub use a2a_config::{
    A2aBlackboardConfig, A2aConfig, A2aConsensusReviewConfig, A2aRoleConfig, A2aRoleMode,
    A2aServerConfig, A2aTaskStoreKind, A2aValidateConfig, A2aWorkflowsConfig, ValidateBuildPlan,
};
pub use complexity::{
    ComplexityRating, compute_cognitive_complexity, compute_cyclomatic_complexity,
};
pub use config::{
    CortexConfig, DEFAULT_INDEXER_PARSE_BATCH_SIZE, IndexingProfile, IndexingSettings, LlmConfig,
    PoolConfig, RerankWeightsConfig, VectorConfig, active_indexing_profile_from_env,
    default_write_pool_size, indexing_settings, migrate_config_file, validate_falkordb_uri,
};
pub use error::{CortexError, Result};
pub use git::{
    BlameLine, BranchDiff, CommitInfo, FileChangeType, FileDiff, GitError, GitOperations,
};
pub use ignore::CollectFilesResult;
pub use ignore::{
    CORTEXIGNORE_FILENAME, CortexIgnoreOptions, CortexIgnoreWalker,
    DEFAULT_GLOBAL_CORTEXIGNORE_REL, default_global_cortexignore_path,
    ensure_cortexignore_template, is_policy_excluded,
};
pub use index_scope::{
    canonical_display_path, find_git_repository_root, graph_repository_path_for_index,
};
pub use language::Language;
pub use mcp_config::{McpConfig, McpNetworkConfig, McpProfileKind, McpToolsConfig};
pub use model::{
    CodeEdge, CodeNode, EdgeKind, EntityKind, FreshnessReport, IndexFreshness, IndexedFile,
    Repository, SearchKind,
};
pub use project::{
    BranchIndexInfo, BranchInfo, GitInfo, ProjectConfig, ProjectRef, ProjectState, ProjectStatus,
    ProjectSummary, SyncResult,
};
pub use tokens::{count_tokens, estimate_baseline_from_sample, tokenizer_name};
