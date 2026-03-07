//! # CodeCortex MCP Server Library
//!
//! Model Context Protocol server implementation with 40 production-ready tools.
//!
//! ## Overview
//!
//! This crate implements the MCP server for CodeCortex, providing AI assistants
//! with powerful code intelligence capabilities:
//!
//! - **Code Retrieval**: Context capsules, skeleton views, signature lookup
//! - **Impact Analysis**: Call graphs, blast radius, logic flow tracking
//! - **Code Quality**: Complexity analysis, dead code detection, pattern finding
//! - **Memory System**: Persistent observations, session context
//! - **Project Management**: Multi-project support with Git integration
//! - **Quality Metrics**: Tool reliability, latency tracking, health status
//!
//! ## Tool Categories
//!
//! | Category | Tools |
//! |----------|-------|
//! | Code Retrieval | get_context_capsule, find_code, get_skeleton, get_signature |
//! | Impact Analysis | get_impact_graph, search_logic_flow, find_dead_code |
//! | Code Quality | calculate_cyclomatic_complexity, find_tests, analyze_refactoring, find_patterns |
//! | Diagnostics | diagnose, check_health, index_status, explain_result |
//! | Memory System | save_observation, get_session_context, search_memory |
//! | Project Management | list/add/remove/set/get projects, list_branches, refresh |
//! | LSP Integration | submit_lsp_edges, workspace_setup |
//! | Repository Ops | add_code_to_graph, list/delete repos, get_stats, bundles |
//! | Watch System | watch/unwatch_directory, list_watched_paths |
//! | Advanced | execute_cypher_query, analyze_code_relationships, jobs |
//!
//! ## Performance SLOs
//!
//! | Tool | p50 | p95 | Timeout |
//! |------|-----|-----|---------|
//! | get_context_capsule | 600ms | 2500ms | 8s |
//! | get_impact_graph | 500ms | 2200ms | 8s |
//! | search_logic_flow | 700ms | 3000ms | 10s |
//! | get_skeleton | 50ms | 200ms | 2s |
//!
//! ## Example
//!
//! ```rust,no_run
//! use cortex_mcp::CortexHandler;
//! use cortex_core::config::CortexConfig;
//!
//! // The handler implements the MCP server protocol
//! let handler = CortexHandler::new(CortexConfig::default());
//!
//! // Tools are automatically registered and available to MCP clients
//! let tools = cortex_mcp::tool_names();
//! assert!(!tools.is_empty());
//! assert_eq!(tools.len(), 46);
//! ```
//!
//! ## Quality Metrics
//!
//! ```rust,no_run
//! use cortex_mcp::{QualityRegistry, QualityTimer, QualityHealthStatus};
//!
//! let registry = QualityRegistry::with_defaults();
//!
//! // Time a tool invocation
//! let timer = QualityTimer::new(&registry, "get_context_capsule");
//! // ... execute tool ...
//! // Timer automatically records on drop
//!
//! // Get metrics
//! let metrics = registry.get_metrics("get_context_capsule");
//!
//! // Get system health
//! let health = registry.health_status();
//! ```
//!
//! ## Feature Flags
//!
//! Tools can be enabled/disabled via environment variables:
//!
//! - `CORTEX_FLAG_<TOOL_NAME>_ENABLED=0/1`
//! - Example: `CORTEX_FLAG_MCP_MEMORY_READ_ENABLED=0`

mod cache;
mod capsule;
mod centrality;
pub mod contracts;
mod flags;
pub mod handler;
mod impact;
mod jobs;
mod logic_flow;
mod lsp_ingest;
mod memory;
mod metrics;
mod project_tools;
pub mod quality;
mod telemetry;
mod tfidf;

pub use cache::{CacheHierarchy, CacheStats, L1Cache, L2Cache};
pub use capsule::{
    CapsuleConfig, CapsuleItem, ContextCapsuleBuilder, ContextCapsuleResult, GraphSearchResult,
    IntentWeights,
};
pub use centrality::{CentralityGraph, CentralityScorer, CombinedCentrality, Edge};
pub use contracts::{CacheHit, EnvelopeBuilder, EnvelopeMeta, EnvelopeStatus, ErrorBody};
pub use flags::FeatureFlags;
pub use handler::CortexHandler;
pub use impact::{
    BlastRadius, ImpactGraph, ImpactGraphBuilder, ImpactNode, ImpactNodeType, Provenance,
    RawRelation,
};
pub use jobs::JobRegistry;
pub use logic_flow::{LogicFlowResult, LogicFlowSearcher, RawEdge, ScoredPath};
pub use lsp_ingest::{IngestedEdge, LspEdgeIngester, LspEdgeInput, MergeMode};
pub use memory::{
    AuditEntry, Classification, MemoryStore, Observation, Severity, generate_observation_id,
};
pub use metrics::{
    HealthCheck, HealthCheckStatus, HealthChecker, HealthStatus, LatencySnapshot, MetricsRegistry,
    MetricsSnapshot, TimerGuard, global_metrics,
};
pub use quality::{
    QualityConfig, QualityHealthStatus, QualityRegistry, QualitySummaryReport, QualityTimer,
    ToolQualityMetrics,
};
pub use telemetry::{TelemetryCollector, TelemetryRegistry, ToolTelemetry};
pub use tfidf::{Document, TfIdfScorer, tokenize};

/// Names of all tools this server exposes.
pub fn tool_names() -> &'static [&'static str] {
    &[
        "add_code_to_graph",
        "watch_directory",
        "get_context_capsule",
        "get_impact_graph",
        "search_logic_flow",
        "get_skeleton",
        "get_signature",
        "find_tests",
        "explain_result",
        "analyze_refactoring",
        "diagnose",
        "find_patterns",
        "index_status",
        "workspace_setup",
        "submit_lsp_edges",
        "save_observation",
        "get_session_context",
        "search_memory",
        "find_code",
        "analyze_code_relationships",
        "execute_cypher_query",
        "find_dead_code",
        "calculate_cyclomatic_complexity",
        "list_indexed_repositories",
        "delete_repository",
        "check_job_status",
        "list_jobs",
        "list_watched_paths",
        "unwatch_directory",
        "load_bundle",
        "export_bundle",
        "get_repository_stats",
        "check_health",
        // Project management tools
        "list_projects",
        "add_project",
        "remove_project",
        "set_current_project",
        "get_current_project",
        "list_branches",
        "refresh_project",
        "project_status",
        "project_sync",
        "project_branch_diff",
        "project_queue_status",
        "project_metrics",
    ]
}

#[cfg(test)]
mod tests {
    use super::tool_names;
    use std::collections::HashSet;

    #[test]
    fn exported_tool_names_are_unique() {
        let tools = tool_names();
        let unique: HashSet<_> = tools.iter().copied().collect();
        assert_eq!(tools.len(), unique.len());
    }

    #[test]
    fn tool_names_include_context_capsule() {
        let tools = tool_names();
        assert!(
            tools.contains(&"get_context_capsule"),
            "get_context_capsule must be in tool_names (used by handler and docs)"
        );
    }
}
