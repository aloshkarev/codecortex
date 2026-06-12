//! # CodeCortex MCP Server Library
//!
//! Model Context Protocol server implementation with 46 production-ready tools.
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
//! Tools are enabled by default; opt out via environment variables:
//!
//! - `CORTEX_FLAG_MCP_<TOOL_NAME>_ENABLED=0` (or `1` to force on)
//! - Example: `CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED=0`
//!
//! ## Enterprise defaults
//!
//! - `CORTEX_MCP_PROFILE=strict`: tightens optional MCP surfaces (see [`McpProfile`]).
//! - `CORTEX_MCP_AUDIT_LOG=/path/to.jsonl`: newline-delimited JSON audit for envelope-based tools.

mod a2a_facade;
pub mod a2a_grpc;
mod a2a_host;
pub mod a2a_http;
pub mod a2a_services;
pub mod agent_pack;
mod audit;
mod cache;
pub mod capsule;
mod centrality;
pub mod contracts;
mod flags;
pub mod handler;
mod handler_guides;
pub mod host_guard;
mod impact;
pub mod intelligence;
mod jobs;
mod lazy_tools;
mod logic_flow;
mod lsp_ingest;
mod mcp_profile;
mod mcp_protocol;
mod memory;
mod metrics;
mod network_server;
mod project_tools;
pub mod quality;
mod rerank;
mod response_buffer;
mod savings;
#[allow(deprecated, dead_code)]
mod server;
mod telemetry;
mod tfidf;
mod tools;
mod vector_service;

pub use agent_pack::{
    AgentPackInstallOptions, AgentPackInstallResult, install_agent_pack, resolve_agent_pack,
};
pub use audit::{ToolAuditEvent, log_tool_audit};
pub use cache::{CacheHierarchy, CacheStats, L1Cache, L2Cache};
pub use capsule::{
    CapsuleConfig, CapsuleItem, ContextCapsuleBuilder, ContextCapsuleResult, GraphSearchResult,
    IntentWeights,
};
pub use centrality::{CentralityGraph, CentralityScorer, CombinedCentrality, Edge};
pub use contracts::{
    CacheHit, EnvelopeBuilder, EnvelopeMeta, EnvelopeStatus, ErrorBody, FreshnessState,
    OmittedItem, ResponseScope, SourcePolicy, TokenBudget, TokenSavings,
};
pub use flags::FeatureFlags;
pub use handler::{CortexHandler, McpServeOptions, McpTransport, start_with_options};
pub use impact::{
    BlastRadius, ImpactGraph, ImpactGraphBuilder, ImpactNode, ImpactNodeType, Provenance,
    RawRelation,
};
pub use jobs::JobRegistry;
pub use logic_flow::{LogicFlowResult, LogicFlowSearcher, RawEdge, ScoredPath};
pub use lsp_ingest::{IngestedEdge, LspEdgeIngester, LspEdgeInput, MergeMode};
pub use mcp_profile::McpProfile;
pub use memory::{
    AuditEntry, Classification, MemoryStore, Observation, Severity, generate_observation_id,
};
pub use metrics::{
    HealthCheck, HealthCheckStatus, HealthChecker, HealthStatus, LatencySnapshot, MetricsRegistry,
    MetricsSnapshot, TimerGuard, global_metrics,
};
pub use network_server::{NetworkState, start_network};
pub use quality::{
    QualityConfig, QualityHealthStatus, QualityRegistry, QualitySummaryReport, QualityTimer,
    ToolQualityMetrics,
};
pub use rerank::{RerankCandidate, RerankWeights, content_etag, rerank_candidates};
pub use savings::{
    SavingsBucket, SavingsEvent, SavingsLedger, SavingsReport, SavingsTotals, SessionCounters,
    compute_token_savings, finish_counted_response, flush as flush_savings, init_from_config,
    load_report as load_savings_report, record_call as record_savings_call, reset as reset_savings,
    savings_dir, savings_enabled,
};
pub use telemetry::{TelemetryCollector, TelemetryRegistry, ToolTelemetry};
pub use tfidf::{Bm25Scorer, Document, LexicalMode, TfIdfScorer, rrf_fuse, tokenize};
pub use tools::{
    IndexTier, PrivacyRisk, TimeoutTier, TokenPolicy, ToolCard, ToolCostClass, ToolGuidance,
    ToolMetadata, output_token_hint, tool_cards, tool_guidance_for, tool_metadata,
    tool_metadata_for, tool_names,
};
pub use vector_service::collect_indexable_code_files;
pub use vector_service::{VectorIndexResult, VectorService};

#[cfg(test)]
mod tests {
    use super::{tool_metadata, tool_names};
    use std::collections::HashSet;

    #[test]
    fn exported_tool_names_are_unique() {
        let tools = tool_names();
        let unique: HashSet<_> = tools.iter().copied().collect();
        assert_eq!(tools.len(), unique.len());
    }

    #[test]
    fn exported_tool_metadata_covers_names() {
        let tools: HashSet<_> = tool_names().iter().copied().collect();
        let metadata: HashSet<_> = tool_metadata().iter().map(|meta| meta.name).collect();
        assert_eq!(tools, metadata);
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
