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
//! use cortex_graph::GraphClient;
//! use cortex_core::SearchKind;
//!
//! # async fn example(client: GraphClient) -> cortex_core::Result<()> {
//! let analyzer = Analyzer::new(client);
//!
//! // Find code by name (uses parameterized queries internally for safety)
//! let results = analyzer.find_code("UserRepository", SearchKind::Name, None).await?;
//! # Ok(())
//! # }
//! ```

mod analyzer;
pub mod code_smells;
pub mod context;
pub mod coupling;
pub mod cross_project;
pub mod duplication;
pub mod navigation;
pub mod refactoring;
pub mod review;
pub mod smells;

pub use analyzer::{AnalyzePathFilters, Analyzer};
pub use code_smells::{
    CodeSmell, FunctionMetrics, Severity, SmellCategory, SmellConfig, SmellDetector, SmellType,
};
pub use context::{ProjectAnalysisContext, ProjectSymbolIndex, SymbolLocation};
pub use coupling::{
    CohesionMetrics, CohesionType, CouplingAnalyzer, CouplingMetrics, CouplingRelation,
    CouplingType,
};
pub use cross_project::{
    ApiSurfaceComparison, CrossProjectAnalyzer, CrossProjectLocation, CrossProjectMatch,
    SharedDependency,
};
pub use duplication::{CodeLocation, DuplicateBlock, DuplicationConfig, DuplicationDetector};
pub use navigation::{
    BranchStructuralDiff, DefinitionConfidence, DefinitionResult, ImpactEntry, ModifiedSymbolEntry,
    NavigationEngine, QuickInfo, QuickInfoMetrics, StructuralDiffSummary, SymbolDiffEntry,
    UsageKind, UsageResult, extract_signature_from_source,
};
pub use refactoring::{
    Priority, RefactoringEngine, RefactoringRecommendation, RefactoringTechnique,
};
pub use review::{
    ReviewAnalyzer, ReviewFileInput, ReviewInput, ReviewLineRange, ReviewRefactorFinding,
    ReviewReport, ReviewSmellFinding, ReviewSummary,
};
pub use smells::{
    SmellCategory as SmellsCategory, detect_alternative_classes, detect_comments,
    detect_data_classes, detect_data_clumps, detect_dead_code, detect_dead_code_with_context,
    detect_deep_nesting, detect_divergent_change, detect_duplicate_code,
    detect_duplicate_code_with_context, detect_feature_envy, detect_feature_envy_with_context,
    detect_inappropriate_intimacy, detect_inappropriate_intimacy_with_context,
    detect_large_classes, detect_lazy_classes, detect_long_functions, detect_long_parameter_lists,
    detect_message_chains, detect_middle_man, detect_parallel_inheritance,
    detect_primitive_obsession, detect_refused_bequest, detect_shotgun_surgery,
    detect_shotgun_surgery_with_context, detect_speculative_generality, detect_switch_statements,
    detect_temporary_fields,
};
