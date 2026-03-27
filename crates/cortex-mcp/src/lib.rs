//! # CodeCortex MCP Server Library
//!
//! Model Context Protocol server implementation with a broad set of production-ready tools.
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
//! assert_eq!(tools.len(), cortex_mcp::tool_catalog::TOOL_NAMES.len());
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
mod network_server;
mod project_tools;
pub mod quality;
#[allow(deprecated, dead_code)]
mod server;
mod telemetry;
mod tfidf;
pub mod tool_catalog;
mod vector_service;

pub use cache::{CacheHierarchy, CacheStats, L1Cache, L2Cache};
pub use capsule::{
    CapsuleConfig, CapsuleItem, ContextCapsuleBuilder, ContextCapsuleResult, GraphSearchResult,
    IntentWeights,
};
pub use centrality::{CentralityGraph, CentralityScorer, CombinedCentrality, Edge};
pub use contracts::{CacheHit, EnvelopeBuilder, EnvelopeMeta, EnvelopeStatus, ErrorBody};
pub use flags::FeatureFlags;
pub use handler::{CortexHandler, McpServeOptions, McpTransport, start_with_options};
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
pub use vector_service::{VectorIndexResult, VectorService};

/// Names of all tools this server exposes (alphabetically sorted; same as MCP registration).
#[must_use]
pub fn tool_names() -> &'static [&'static str] {
    tool_catalog::TOOL_NAMES
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

    #[test]
    fn tool_names_include_vector_tools() {
        let tools = tool_names();
        assert!(tools.contains(&"vector_index_repository"));
        assert!(tools.contains(&"vector_search"));
        assert!(tools.contains(&"vector_search_hybrid"));
        assert!(tools.contains(&"vector_index_status"));
    }
}

#[cfg(test)]
mod tool_metadata_contract {
    use super::tool_catalog::{self, ToolHints};
    use crate::handler::CortexHandler;
    use cortex_core::config::CortexConfig;
    use rmcp::{ServerHandler, model::Tool};

    #[test]
    fn initialize_instructions_include_resource_uri() {
        let h = CortexHandler::new(CortexConfig::default());
        let info = h.get_info();
        let inst = info
            .instructions
            .as_ref()
            .expect("server should expose instructions");
        assert!(inst.len() > 200, "instructions should be substantive");
        assert!(
            inst.contains(tool_catalog::TOOL_ROUTING_RESOURCE_URI),
            "instructions should point agents at the routing resource"
        );
    }

    #[test]
    fn tool_definitions_align_with_catalog() {
        let h = CortexHandler::new(CortexConfig::default());
        let mut tools: Vec<Tool> = h.tool_definitions_for_test();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(
            tools.len(),
            tool_catalog::TOOL_NAMES.len(),
            "handler and catalog must expose the same tool count"
        );
        for (tool, expected_name) in tools.iter().zip(tool_catalog::TOOL_NAMES.iter()) {
            assert_eq!(
                tool.name.as_ref(),
                *expected_name,
                "tool list ordering / membership drift"
            );
            let desc = tool.description.as_deref().unwrap_or("");
            assert!(
                desc.len() >= 48,
                "tool {:?} should have a substantive description",
                tool.name
            );
            let hints: ToolHints = tool_catalog::hints_for(expected_name)
                .unwrap_or_else(|| panic!("catalog missing hints for {expected_name}"));
            let ann = tool
                .annotations
                .as_ref()
                .unwrap_or_else(|| panic!("tool {:?} missing annotations", tool.name));
            assert_eq!(
                ann.read_only_hint,
                Some(hints.read_only),
                "read_only_hint mismatch for {:?}",
                tool.name
            );
            assert_eq!(
                ann.open_world_hint,
                Some(hints.open_world),
                "open_world_hint mismatch for {:?}",
                tool.name
            );
            if hints.destructive {
                assert_eq!(
                    ann.destructive_hint,
                    Some(true),
                    "expected destructive for {:?}",
                    tool.name
                );
            } else {
                assert_ne!(
                    ann.destructive_hint,
                    Some(true),
                    "unexpected destructive for {:?}",
                    tool.name
                );
            }
            // idempotent_hint is only meaningful when read_only_hint is false (per MCP notes).
            if !hints.read_only {
                if hints.idempotent {
                    assert_eq!(
                        ann.idempotent_hint,
                        Some(true),
                        "expected idempotent for {:?}",
                        tool.name
                    );
                } else {
                    assert_ne!(
                        ann.idempotent_hint,
                        Some(true),
                        "unexpected idempotent for {:?}",
                        tool.name
                    );
                }
            }
        }
    }
}
