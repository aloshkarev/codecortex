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

pub mod complexity;
pub mod config;
pub mod error;
pub mod git;
pub mod language;
pub mod model;
pub mod project;

pub use complexity::compute_cyclomatic_complexity;
pub use config::CortexConfig;
pub use error::{CortexError, Result};
pub use git::{
    BlameLine, BranchDiff, CommitInfo, FileChangeType, FileDiff, GitError, GitOperations,
};
pub use language::Language;
pub use model::{CodeEdge, CodeNode, EdgeKind, EntityKind, IndexedFile, Repository, SearchKind};
pub use project::{
    BranchIndexInfo, BranchInfo, GitInfo, ProjectConfig, ProjectRef, ProjectState, ProjectStatus,
    ProjectSummary, SyncResult,
};
