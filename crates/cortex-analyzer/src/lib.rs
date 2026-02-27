//! # CodeCortex Analyzer Library
//!
//! Code analysis queries for call graphs, dependencies, and code metrics.
//!
//! ## Overview
//!
//! This crate provides analysis functionality:
//!
//! - **Analyzer**: [`Analyzer`] for building Cypher queries for code analysis
//! - **Search Types**: Find by name, pattern, type, or content
//! - **Call Graphs**: Find callers, callees, and call chains
//! - **Code Metrics**: Complexity analysis, dead code detection
//! - **Code Smells**: [`SmellDetector`] detects long functions, deep nesting, magic numbers
//! - **Coupling Analysis**: [`CouplingAnalyzer`] measures module dependencies
//! - **Cohesion Metrics**: [`CohesionMetrics`] measures how well module elements belong together
//! - **Duplication Detection**: [`DuplicationDetector`] finds duplicate code blocks
//!
//! ## Query Types
//!
//! | Query Type | Description |
//! |------------|-------------|
//! | `find_code` | Search symbols by name/pattern/type/content |
//! | `callers` | Find functions that call a target |
//! | `callees` | Find functions called by a target |
//! | `call_chain` | Find path between two symbols |
//! | `dead_code` | Find unreachable code |
//! | `complexity` | Calculate cyclomatic complexity |
//!
//! ## Code Quality Analysis
//!
//! ```rust
//! use cortex_analyzer::{SmellDetector, SmellConfig, CouplingAnalyzer, DuplicationDetector};
//!
//! // Detect code smells
//! let detector = SmellDetector::new();
//! let smells = detector.detect("fn long_fn() { /* ... */ }", "test.rs");
//!
//! // Analyze coupling
//! let mut coupling = CouplingAnalyzer::new();
//! coupling.add_dependency("module_a", "module_b");
//! let metrics = coupling.analyze_coupling("module_a");
//!
//! // Find duplicates
//! let dup_detector = DuplicationDetector::new();
//! let sources = vec![("a.rs".to_string(), "code...".to_string())];
//! let duplicates = dup_detector.find_duplicates(&sources);
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use cortex_analyzer::Analyzer;
//! use cortex_core::SearchKind;
//!
//! let analyzer = Analyzer::new();
//!
//! // Build a query to find code by name
//! let query = analyzer.build_find_code_query(
//!     "UserRepository",
//!     SearchKind::Name,
//!     None
//! );
//! ```

mod analyzer;
pub mod code_smells;
pub mod coupling;
pub mod duplication;

pub use analyzer::Analyzer;
pub use code_smells::{CodeSmell, FunctionMetrics, Severity, SmellConfig, SmellDetector, SmellType};
pub use coupling::{
    CohesionMetrics, CohesionType, CouplingAnalyzer, CouplingMetrics, CouplingRelation, CouplingType,
};
pub use duplication::{
    CodeLocation, DuplicateBlock, DuplicationConfig, DuplicationDetector,
};
