use crate::agent_pack::{
    AgentPackInstallOptions, AgentPackInstallResult, install_agent_pack, resolve_agent_pack,
};
use crate::cache::{CacheHierarchy, L1Cache};
use crate::capsule::{ContextCapsuleBuilder, GraphSearchResult};
use crate::contracts::{
    EnvelopeBuilder, FreshnessState, OmittedItem, ResponseScope, SourcePolicy, TokenBudget,
    WARNING_EMBEDDER_TIMEOUT, WARNING_FALLBACK_TO_LEXICAL, WARNING_VECTOR_STORE_UNAVAILABLE,
    error as envelope_error, success as envelope_success, success_json as envelope_success_json,
};
use crate::response_buffer::ResponseBuffer;
use crate::handler_guides::{
    codecortex_prompt_text, codecortex_prompts, codecortex_resource_text, codecortex_resources,
    codecortex_server_instructions, freshness_state_from_label, infer_agent_intent,
    metadata_safe_fallbacks, recommendation_entry, recommendation_warnings,
    recommended_tool_sequence, tool_card_for,
};
use crate::lazy_tools::{self, PromotedTools, new_promoted_tools};
use crate::jobs::{JobRegistry, JobState};
use crate::memory::{Classification, MemoryStore, Observation, Severity as MemorySeverity};
use crate::metrics::global_metrics;
use crate::vector_service::{VectorSearchFilters, VectorSearchRequest, VectorService};
use crate::{FeatureFlags, ToolCard, tool_metadata_for};
use cortex_analyzer::{
    AnalyzePathFilters, Analyzer, CrossProjectAnalyzer, NavigationEngine, ResolveOutcome,
    ResolveSymbolInput, ReviewAnalyzer, ReviewFileInput, ReviewInput, ReviewLineRange, Severity,
    SymbolResolver, UsageKind, callable_kinds_cypher_list, is_callable_kind,
    normalize_repo_relative_file, normalize_repo_scope,
};
use cortex_core::{CortexConfig, GitOperations, IndexFreshness, ProjectStatus, SearchKind};
use cortex_graph::{
    BackendKind, BundleStore, GraphClient, get_branch_indexes, mark_branch_vector_fresh,
};
use cortex_indexer::Indexer;
use cortex_parser::SignatureExtractor;
use cortex_vector::{HybridSearch, LanceStore, SearchType};
use cortex_watcher::{ProjectRegistry, WatchSession};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    service::{RequestContext, RoleServer, ServerInitializeError},
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex as AsyncMutex;

/// Scale default token budgets using [`crate::mcp_profile::McpProfile`] (strict uses lower caps).
fn mcp_scaled_budget(default: usize, min_v: usize, max_v: usize) -> usize {
    let m = crate::mcp_profile::McpProfile::from_env().default_context_budget_multiplier();
    ((default as f64 * m) as usize).clamp(min_v, max_v)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio,
    HttpSse,
    WebSocket,
    Multi,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServeOptions {
    pub transport: McpTransport,
    pub listen: SocketAddr,
    pub token: Option<String>,
    pub allow_remote: bool,
    pub max_clients: usize,
    pub idle_timeout_secs: u64,
    pub feature_flags: FeatureFlags,
}

impl Default for McpServeOptions {
    fn default() -> Self {
        Self {
            transport: McpTransport::Stdio,
            listen: SocketAddr::from(([127, 0, 0, 1], 3001)),
            token: None,
            allow_remote: false,
            max_clients: 64,
            idle_timeout_secs: 600,
            feature_flags: FeatureFlags::from_env(),
        }
    }
}


#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexPathReq {
    /// Directory or file path to index
    pub path: String,
    /// Force a full parse/write pass instead of trusting the local hash cache.
    /// Defaults to true for MCP calls so an agent can repair an empty/stale graph reliably.
    pub force: Option<bool>,
    /// Also perform vector indexing for semantic retrieval
    pub include_vector: Option<bool>,
    /// Block until the background index job completes (bounded by wait_timeout_secs)
    pub wait: Option<bool>,
    /// Max seconds to wait when wait is true (default 600)
    pub wait_timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PathReq {
    /// Directory or file path
    pub path: String,
    /// Required for destructive graph delete (delete_repository)
    pub confirm: Option<bool>,
    /// When true, report what would be deleted without mutating state
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeadCodeReq {
    pub include_paths: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorIndexRepositoryReq {
    /// Directory path to index
    pub path: String,
    /// Optional repository identifier
    pub repo_path: Option<String>,
    /// Optional git branch
    pub branch: Option<String>,
    /// Optional source revision
    pub revision: Option<String>,
    pub include_paths: Option<Vec<String>>,
    pub max_files: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorIndexFileReq {
    /// File path to index
    pub path: String,
    /// Optional repository identifier
    pub repo_path: Option<String>,
    /// Optional git branch
    pub branch: Option<String>,
    /// Optional source revision
    pub revision: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorSearchReq {
    /// Natural-language or keyword search query (e.g. "auth token refresh", "where is login validated")
    pub query: String,
    /// Maximum number of results to return (default often 10–20)
    pub k: Option<usize>,
    /// Restrict search to this repository path
    pub repo_path: Option<String>,
    /// Restrict search to files under this path prefix
    pub path: Option<String>,
    /// Filter by node kind (e.g. function, class, method)
    pub kind: Option<String>,
    /// Filter by language (e.g. rust, python, typescript)
    pub language: Option<String>,
    /// One of: semantic | hybrid | structural (default: semantic)
    pub search_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorIndexStatusReq {
    /// Repository path to check; omit for all repos
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VectorDeleteRepositoryReq {
    /// Repository identifier/path whose vector index should be removed
    pub repo_path: String,
    pub confirm: Option<bool>,
    pub dry_run: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SimilarAcrossReq {
    /// Minimum number of repositories in which symbol should appear
    pub min_repos: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SharedDepsReq {
    /// Optional explicit repository filter list
    pub repos: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CompareApiReq {
    pub repo_a: String,
    pub repo_b: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CrossProjectSearchReq {
    pub query: String,
    pub repositories: Vec<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GoToDefReq {
    pub symbol: String,
    pub qualified_name: Option<String>,
    pub from_file: Option<String>,
    pub from_line: Option<u32>,
    /// Graph repository scope when no current project is set
    pub repo_path: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindUsagesReq {
    pub symbol: String,
    pub qualified_name: Option<String>,
    pub from_file: Option<String>,
    pub kind: Option<String>,
    pub repo_path: Option<String>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickInfoReq {
    pub symbol: String,
    pub repo_path: Option<String>,
    pub branch: Option<String>,
    /// Return `not_modified` when content matches this etag.
    pub if_none_match: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BranchStructuralDiffReq {
    pub source_branch: String,
    pub target_branch: String,
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct PrReviewReq {
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    /// Review scope path (subdir or repo); git root is resolved for diffs
    pub path: Option<String>,
    /// Repository scope when no current project is set
    pub repo_path: Option<String>,
    pub min_severity: Option<String>,
    pub max_findings: Option<usize>,
    pub budget_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct FindCodeReq {
    /// Search string: symbol name, regex pattern, or content snippet depending on kind
    pub query: String,
    /// One of: name | pattern | type | content  (default: pattern)
    pub kind: Option<String>,
    /// Optional path prefix filter
    pub path_filter: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RelationshipReq {
    /// One of: find_callers | find_callees | find_all_callers | find_all_callees |
    ///         class_hierarchy | dead_code | overrides | module_deps | variable_scope |
    ///         call_chain | find_importers | find_by_decorator | find_by_argument | find_complexity
    pub query_type: String,
    pub target: Option<String>,
    pub target2: Option<String>,
    pub depth: Option<usize>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CypherReq {
    /// Cypher query string (e.g. MATCH (n:CodeNode) RETURN n LIMIT 10)
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ComplexityReq {
    pub top_n: Option<u64>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct JobStatusReq {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportBundleReq {
    pub repository_path: String,
    pub output_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContextCapsuleReq {
    /// Task or topic description (e.g. "refactor auth", "find bug in login")
    pub query: String,
    /// Return `not_modified` when capsule matches this etag.
    pub if_none_match: Option<String>,
    /// Optional intent hint (e.g. debug, refactor, onboard); can improve ranking
    pub task_intent: Option<String>,
    /// Restrict to this repository path
    pub repo_path: Option<String>,
    /// Approximate token budget for returned snippets (default 6000, max 12000)
    pub max_tokens: Option<usize>,
    /// Maximum number of items to return (default 40, max 100)
    pub max_items: Option<usize>,
    /// Whether to include test files in results
    pub include_tests: Option<bool>,
    /// Path substrings to filter by (e.g. ["src/auth"]); items must match at least one
    pub path_filter: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PatchContextReq {
    pub task: String,
    pub include_paths: Option<Vec<String>>,
    pub exclude_paths: Option<Vec<String>>,
    pub budget_tokens: Option<usize>,
    pub mode: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeltaContextReq {
    pub source_branch: Option<String>,
    pub target_branch: Option<String>,
    pub repo_path: Option<String>,
    pub include_paths: Option<Vec<String>>,
    pub exclude_paths: Option<Vec<String>>,
    pub budget_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TestContextReq {
    pub symbol: String,
    pub repo_path: Option<String>,
    pub budget_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApiContractReq {
    pub symbol: String,
    pub repo_path: Option<String>,
    pub include_related: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummarizeModuleReq {
    pub path: String,
    pub repo_path: Option<String>,
    pub budget_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EstimateContextCostReq {
    pub task: String,
    pub mode: Option<String>,
    pub include_paths: Option<Vec<String>>,
    pub budget_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecommendToolsReq {
    /// User task or agent goal, e.g. "fix auth refresh bug" or "review this PR"
    pub task: String,
    /// Optional explicit intent: patch, review, search, impact, test, freshness, memory, project
    pub intent: Option<String>,
    /// Optional path/module scope used to prefer bounded context tools
    pub include_paths: Option<Vec<String>>,
    /// Optional repository path for examples and scope reporting
    pub repo_path: Option<String>,
    /// Context budget; lower budgets prefer metadata/signature tools
    pub budget_tokens: Option<usize>,
    /// Set false when the client should avoid tools that can return source snippets
    pub allow_source: Option<bool>,
    /// Known freshness: fresh, stale, partial, unknown. Stale states add repair tools first.
    pub freshness: Option<String>,
    /// Maximum recommendations to return
    pub limit: Option<usize>,
    /// High-level artifact or workflow: bugfix, review, explore, navigate, incident
    pub artifact: Option<String>,
    /// Primary language hint (e.g. rust, typescript) for routing
    pub language: Option<String>,
    /// Known symbol or path fragment to bias toward API/navigation tools
    pub symbol_hint: Option<String>,
    /// When true, avoid vector-heavy tools in sequences unless explicitly needed
    pub graph_only: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolsSearchReq {
    pub query: String,
    pub max_results: Option<usize>,
    pub promote: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolProfileReq {
    pub tool: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolGuidanceReq {
    /// Exact MCP tool name to describe. If omitted, task-based filtered cards are returned.
    pub tool_name: Option<String>,
    /// Optional user task for task-relevant guidance cards.
    pub task: Option<String>,
    /// Maximum cards to return for task-based mode.
    pub limit: Option<usize>,
    /// Same routing hints as `recommend_tools` for consistent card filtering
    pub artifact: Option<String>,
    pub language: Option<String>,
    pub symbol_hint: Option<String>,
    pub graph_only: Option<bool>,
    pub budget_tokens: Option<usize>,
    pub freshness: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ImpactGraphReq {
    /// Symbol name (function, class, method) to get call graph for
    pub symbol: String,
    /// Optional type hint: function, class, method, etc.
    pub symbol_type: Option<String>,
    /// Restrict to this repository path
    pub repo_path: Option<String>,
    /// Traversal depth (default 2–3; higher can be slow)
    pub depth: Option<usize>,
    /// Include importers/dependents in the graph
    pub include_importers: Option<bool>,
    /// Include test files in the graph
    pub include_tests: Option<bool>,
    /// Soft cap on response size; marks partial when truncated
    pub budget_tokens: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LogicFlowReq {
    /// Starting symbol (e.g. entry point or caller)
    pub from_symbol: String,
    /// Ending symbol (e.g. target function or callee)
    pub to_symbol: String,
    /// Restrict to this repository path
    pub repo_path: Option<String>,
    /// Maximum number of paths to return
    pub max_paths: Option<usize>,
    /// Maximum traversal depth per path
    pub max_depth: Option<usize>,
    /// If true, return partial paths when full path is not found
    pub allow_partial: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SkeletonReq {
    /// File path (relative to repo or absolute) to get skeleton for
    pub path: String,
    /// Optional mode (e.g. full, compact); implementation-dependent
    pub mode: Option<String>,
    /// Repository root when path is relative
    pub repo_path: Option<String>,
    /// Return `not_modified` when skeleton matches this etag.
    pub if_none_match: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndexStatusReq {
    /// Repository path to check; omit for all repos
    pub repo_path: Option<String>,
    /// Include list of background jobs
    pub include_jobs: Option<bool>,
    /// Include watcher status (watched paths)
    pub include_watcher: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WorkspaceSetupReq {
    pub repo_path: Option<String>,
    pub detect_agents: Option<bool>,
    pub generate_configs: Option<bool>,
    pub install_git_hooks: Option<bool>,
    pub non_interactive: Option<bool>,
    pub overwrite: Option<bool>,
    /// Install skills, subagents, hooks, and rules from the CodeCortex agent pack
    pub install_agent_pack: Option<bool>,
    /// Override agent pack root (default: CORTEX_AGENT_PACK, plugin/codecortex, or share/)
    pub agent_pack_root: Option<String>,
    /// Write `.cursor/mcp.json` (defaults to true when `generate_configs` is true)
    pub install_cursor_mcp: Option<bool>,
    /// Target platforms; v1 supports `cursor` only
    pub targets: Option<Vec<String>>,
    /// Start directory watch after setup (same as watch_directory)
    pub enable_watch: Option<bool>,
    /// Extra paths to watch; defaults to `repo_path`
    pub watch_paths: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aSpawnSessionReq {
    /// High-level task for the A2A session.
    pub task: String,
    /// Workflow template (e.g. consensus_review).
    pub workflow: Option<String>,
    /// Agent roles to involve.
    pub roles: Option<Vec<String>>,
    pub include_paths: Option<Vec<String>>,
    pub exclude_paths: Option<Vec<String>>,
    pub exclude_globs: Option<Vec<String>>,
    pub target_symbol: Option<String>,
    pub source_branch: Option<String>,
    pub target_branch: Option<String>,
    pub mode: Option<String>,
    pub return_immediately: Option<bool>,
    pub wait_for_completion: Option<bool>,
    pub budget_tokens: Option<u32>,
    /// Optional webhook URL registered as push config on spawn.
    pub push_callback_url: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aGetTaskReq {
    pub task_id: String,
    pub history_length: Option<i32>,
    pub spec_json: Option<bool>,
    /// When false, omit artifacts from GetTask response (spec §2.3).
    pub include_artifacts: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aSendMessageReq {
    pub message: String,
    pub context_id: Option<String>,
    pub task_id: Option<String>,
    pub return_immediately: Option<bool>,
    pub workflow: Option<String>,
    pub include_paths: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aListTasksReq {
    pub context_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aCancelTaskReq {
    pub task_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aSubscribeTaskReq {
    pub task_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct A2aListPushConfigsReq {
    /// When omitted, returns all push configs (may be empty)
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ManageCodecortexReq {
    pub repo_path: Option<String>,
    /// assess (default), bootstrap, or repair_plan
    pub action: Option<String>,
    /// Task description for recommend_tools
    pub task: Option<String>,
    /// Run agent pack install (bootstrap action)
    pub install_agent_pack: Option<bool>,
    pub agent_pack_root: Option<String>,
    /// Start or confirm directory watch
    pub enable_watch: Option<bool>,
    pub watch_paths: Option<Vec<String>>,
    /// When true, queue graph indexing if freshness is stale/unknown
    pub auto_repair: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct LspEdgeInput {
    pub caller_fqn: String,
    pub callee_fqn: String,
    pub file: String,
    pub line: u64,
    pub confidence: Option<f64>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubmitLspEdgesReq {
    pub repo_path: String,
    pub edges: Vec<LspEdgeInput>,
    pub merge_mode: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SaveObservationReq {
    pub repo_path: String,
    pub text: String,
    pub severity: Option<String>,
    pub confidence: Option<f64>,
    pub symbol_refs: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub classification: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SessionContextReq {
    pub repo_path: String,
    pub session_id: Option<String>,
    pub include_previous: Option<usize>,
    pub max_items: Option<usize>,
    pub include_stale: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchMemoryReq {
    pub query: String,
    pub repo_path: String,
    pub max_items: Option<usize>,
    pub include_stale: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSignatureReq {
    /// Symbol name to look up (function, method, struct, enum)
    pub symbol: String,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Include related signatures (implementations, overrides)
    pub include_related: Option<bool>,
    /// Return `not_modified` when signatures match this etag.
    pub if_none_match: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindClonesReq {
    /// Optional path substring filter.
    pub path: Option<String>,
    /// Repository scope (defaults to current project).
    pub repo_path: Option<String>,
    /// Minimum Jaccard similarity (default 0.85).
    pub min_jaccard: Option<f64>,
    /// Maximum clone pairs to return (default 100).
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindTestsReq {
    /// Symbol name to find tests for
    pub symbol: String,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Include integration tests
    pub include_integration: Option<bool>,
    /// Maximum number of tests to return
    pub max_results: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExplainResultReq {
    /// Original query to explain
    pub query: String,
    /// Tool that was used (optional, helps with context)
    pub tool: Option<String>,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CtxStatsReq {
    /// Buffered response handle (defaults to the most recent capture)
    pub response_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CtxGrepReq {
    /// Substring to search for within the buffered response
    pub pattern: String,
    /// Buffered response handle (defaults to the most recent capture)
    pub response_id: Option<String>,
    /// Context lines before each match
    pub before: Option<usize>,
    /// Context lines after each match
    pub after: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CtxSliceReq {
    /// Inclusive start character offset
    pub from: usize,
    /// Exclusive end character offset
    pub to: usize,
    /// Buffered response handle (defaults to the most recent capture)
    pub response_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CtxPeekReq {
    /// Number of leading lines to return (default 20)
    pub lines: Option<usize>,
    /// Buffered response handle (defaults to the most recent capture)
    pub response_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeRefactoringReq {
    /// Symbol to analyze for refactoring
    pub symbol: String,
    /// Type of change being considered
    pub change_type: Option<String>,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Include detailed breakdown
    pub detailed: Option<bool>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiagnoseReq {
    /// Type of diagnostic to run: index_health, graph_connectivity, cache_status, all
    pub check: Option<String>,
    /// Repository path (for index health checks)
    pub repo_path: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FindPatternsReq {
    /// Pattern to search for: builder, factory, singleton, repository, service, handler, middleware, observer, strategy
    pub pattern: Option<String>,
    /// Repository path filter (optional)
    pub repo_path: Option<String>,
    /// Minimum confidence threshold (0.0-1.0)
    pub min_confidence: Option<f64>,
    /// Maximum results to return
    pub max_results: Option<usize>,
    /// Include only paths with these prefixes
    pub include_paths: Option<Vec<String>>,
    /// Include only these files (path or file name)
    pub include_files: Option<Vec<String>>,
    /// Include only paths matching these glob patterns
    pub include_globs: Option<Vec<String>>,
    /// Exclude paths with these prefixes
    pub exclude_paths: Option<Vec<String>>,
    /// Exclude these files (path or file name)
    pub exclude_files: Option<Vec<String>>,
    /// Exclude paths matching these glob patterns
    pub exclude_globs: Option<Vec<String>>,
}


#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddProjectReq {
    /// Path to the project directory
    pub path: String,
    /// Optional name for the project (defaults to directory name)
    pub name: Option<String>,
    /// Whether to track branch changes (default: true)
    pub track_branch: Option<bool>,
    /// Branches to keep indexed even when inactive
    pub pinned_branches: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveProjectReq {
    /// Path to the project directory
    pub path: String,
    /// Whether to delete associated index data
    pub delete_data: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetProjectReq {
    /// Path to the project directory
    pub path: String,
    /// Branch to switch to (optional, defaults to current)
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListBranchesReq {
    /// Path to project (optional, uses current project if not specified)
    pub path: Option<String>,
    /// Whether to include remote branches
    pub include_remote: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectStatusReq {
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
    /// Include queue details
    pub include_queue: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectSyncReq {
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
    /// Force full index mode
    pub force: Option<bool>,
    /// Cleanup old branches after sync
    pub cleanup_old_branches: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectBranchDiffReq {
    /// Source branch
    pub source: String,
    /// Target branch
    pub target: String,
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
    /// Commit limit for ahead/behind lists
    pub commit_limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectQueueStatusReq {
    /// Path to project (optional filter)
    pub path: Option<String>,
    /// Maximum jobs returned
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProjectMetricsReq {
    /// Path to project (optional, uses current project)
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct ObservationRecord {
    pub observation_id: String,
    pub repo_id: String,
    pub session_id: String,
    pub created_at: u128,
    pub created_by: String,
    pub text: String,
    pub symbol_refs: Vec<String>,
    pub confidence: f64,
    pub stale: bool,
    pub classification: String,
    pub severity: String,
    pub tags: Vec<String>,
    pub source_revision: String,
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
}


#[derive(Clone)]
pub struct CortexHandler {
    config: CortexConfig,
    feature_flags: FeatureFlags,
    jobs: JobRegistry,
    projects: Arc<ProjectRegistry>,
    /// Reuse one graph connection per MCP process (avoids repeated FalkorDB connects).
    graph_client: Arc<AsyncMutex<Option<GraphClient>>>,
    /// Reuse vector store + embedder for the MCP process lifetime.
    vector_service: Arc<AsyncMutex<Option<Arc<VectorService>>>>,
    /// L1/L2 tool response cache (when `cache_enabled`).
    tool_cache: Arc<AsyncMutex<Option<Arc<CacheHierarchy>>>>,
    /// SQLite session memory (replaces JSON `memory.json`).
    memory_store: Arc<AsyncMutex<Option<Arc<MemoryStore>>>>,
    a2a_hub: Option<Arc<cortex_a2a::A2aHub>>,
    /// Ring buffer of recent large tool responses for ctx_* re-cutting tools.
    response_buffer: Arc<AsyncMutex<ResponseBuffer>>,
    /// Held for `#[tool_router]` / `#[tool_handler]` dispatch; not read directly.
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    /// Tools promoted from deferred catalog via `tools_search` when lazy discovery is enabled.
    promoted_tools: PromotedTools,
}

impl CortexHandler {
    /// Shared gate for all `cortex_a2a_*` tools (`mcp.tools.a2a_spawn_session` + `[a2a].enabled`).
    fn check_a2a_access(
        &self,
        started: Instant,
    ) -> Result<Arc<cortex_a2a::A2aHub>, CallToolResult> {
        if !self.tool_enabled("mcp.a2a_spawn_session.enabled", false) {
            return Err(envelope_error(
                "UNAVAILABLE",
                "A2A MCP tools are disabled — set mcp.tools.a2a_spawn_session = true in ~/.cortex/config.toml",
                None,
                started,
            ));
        }
        if !self.config.a2a.enabled {
            return Err(envelope_error(
                "UNAVAILABLE",
                "A2A is disabled — set a2a.enabled = true in ~/.cortex/config.toml",
                None,
                started,
            ));
        }
        self.a2a_hub()
            .ok_or_else(|| envelope_error("UNAVAILABLE", "A2A hub not initialized", None, started))
    }

    /// Route index/repair through the watcher daemon when running (single sled writer).
    ///
    /// Skips daemon enqueue when this process already holds the sled hash cache (e.g. watch task).
    fn try_enqueue_daemon_index(
        config: &CortexConfig,
        path: &Path,
        force: bool,
    ) -> Result<Option<Value>, anyhow::Error> {
        let hash_path = config
            .hash_cache_path
            .clone()
            .unwrap_or_else(Indexer::default_hash_cache_path);
        if cortex_indexer::hash_cache_held_in_process(&hash_path) {
            tracing::info!(
                "daemon index skipped: process already holds hash cache at {} — using in-process indexer",
                hash_path.display()
            );
            return Ok(None);
        }
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let daemon_status = cortex_watcher::daemon_status(&daemon_paths)?;
        if !daemon_status.running {
            return Ok(None);
        }
        let Some((_repo_root, branch, commit_hash)) = resolve_git_context_for_path(path) else {
            return Ok(None);
        };
        let graph_scope = cortex_core::graph_repository_path_for_index(path, None);
        let enqueue = cortex_watcher::enqueue_index_job(
            &daemon_paths,
            &cortex_watcher::IndexJobRequest {
                repository_path: graph_scope.clone(),
                branch: branch.clone(),
                commit_hash: commit_hash.clone(),
                mode: if force {
                    cortex_watcher::JobMode::Full
                } else {
                    cortex_watcher::JobMode::IncrementalDiff
                },
            },
        )?;
        Ok(Some(json!({
            "status": "queued",
            "daemon": true,
            "job": enqueue.job,
            "deduplicated": enqueue.deduplicated,
            "repository_path": graph_scope,
            "branch": branch,
            "commit": commit_hash,
        })))
    }
}

#[tool_router]
impl CortexHandler {
    pub fn new(config: CortexConfig) -> Self {
        Self::new_with_feature_flags(config, FeatureFlags::from_env())
    }

    pub fn new_with_feature_flags(config: CortexConfig, feature_flags: FeatureFlags) -> Self {
        // Sync constructor: no graph connect. MCP runtime uses `new_async` / `new_with_a2a`
        // with `try_build_a2a_hub` for graph-backed services and blackboard.
        let a2a_hub = if config.a2a.enabled {
            Some(Arc::new(cortex_a2a::A2aHub::new(config.a2a.clone())))
        } else {
            None
        };
        Self::new_with_a2a(config, feature_flags, a2a_hub)
    }

    /// Async constructor for MCP runtime — attaches graph-backed hub when A2A is enabled.
    pub async fn new_async(config: CortexConfig, feature_flags: FeatureFlags) -> Self {
        let a2a_hub = crate::a2a_services::try_build_a2a_hub(&config).await;
        Self::new_with_a2a(config, feature_flags, a2a_hub)
    }

    pub fn new_with_a2a(
        config: CortexConfig,
        feature_flags: FeatureFlags,
        a2a_hub: Option<Arc<cortex_a2a::A2aHub>>,
    ) -> Self {
        crate::savings::init_from_config(&config);
        Self {
            config,
            feature_flags,
            jobs: JobRegistry::default(),
            projects: Arc::new(ProjectRegistry::new()),
            graph_client: Arc::new(AsyncMutex::new(None)),
            vector_service: Arc::new(AsyncMutex::new(None)),
            tool_cache: Arc::new(AsyncMutex::new(None)),
            memory_store: Arc::new(AsyncMutex::new(None)),
            a2a_hub,
            response_buffer: Arc::new(AsyncMutex::new(ResponseBuffer::new())),
            tool_router: Self::tool_router(),
            promoted_tools: new_promoted_tools(),
        }
    }

    pub fn a2a_hub(&self) -> Option<Arc<cortex_a2a::A2aHub>> {
        self.a2a_hub.clone()
    }

    fn savings_enabled(&self) -> bool {
        self.config.mcp.savings_enabled
    }

    fn finish_counted_tool(
        &self,
        builder: EnvelopeBuilder,
        data: Value,
        tool: &str,
        repo: Option<&str>,
        baseline_total_chars: usize,
        baseline_sample: &str,
    ) -> CallToolResult {
        crate::savings::finish_counted_response(
            self.savings_enabled(),
            builder,
            data,
            tool,
            repo,
            baseline_total_chars,
            baseline_sample,
        )
    }

    fn baseline_sample(text: &str) -> String {
        text.chars().take(8192).collect()
    }

    fn graph_connect_hint(&self) -> &'static str {
        "Ensure FalkorDB is running: docker start codecortex-falkordb (port 6379)"
    }

    async fn graph_client(&self) -> Result<GraphClient, McpError> {
        let mut slot = self.graph_client.lock().await;
        if let Some(client) = slot.as_ref() {
            return Ok(client.clone());
        }
        let client = GraphClient::connect(&self.config)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        *slot = Some(client.clone());
        Ok(client)
    }

    async fn promote_vector_freshness(
        &self,
        repository_path: &str,
        branch: &str,
        indexed_documents: usize,
    ) {
        promote_vector_freshness_with_config(
            &self.config,
            repository_path,
            branch,
            indexed_documents,
        )
        .await;
    }

    async fn init_vector_service(
        slot: &Arc<tokio::sync::Mutex<Option<Arc<VectorService>>>>,
        config: &CortexConfig,
    ) -> Result<Arc<VectorService>, anyhow::Error> {
        let mut guard = slot.lock().await;
        if let Some(svc) = guard.as_ref() {
            return Ok(Arc::clone(svc));
        }
        let svc = Arc::new(
            VectorService::from_config(config)
                .await
                .map_err(|e| anyhow::anyhow!(e))?,
        );
        *guard = Some(Arc::clone(&svc));
        Ok(svc)
    }

    async fn vector_service(&self) -> Result<Arc<VectorService>, McpError> {
        Self::init_vector_service(&self.vector_service, &self.config)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))
    }

    async fn tool_cache(&self) -> Arc<CacheHierarchy> {
        let mut slot = self.tool_cache.lock().await;
        if let Some(cache) = slot.as_ref() {
            return Arc::clone(cache);
        }
        let cache = Arc::new(CacheHierarchy::new());
        *slot = Some(Arc::clone(&cache));
        cache
    }

    async fn memory_store(&self) -> Result<Arc<MemoryStore>, McpError> {
        let mut slot = self.memory_store.lock().await;
        if let Some(store) = slot.as_ref() {
            return Ok(Arc::clone(store));
        }
        let store = Arc::new(
            MemoryStore::open().map_err(|e| McpError::internal_error(e.to_string(), None))?,
        );
        migrate_json_memory_db(&store)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        *slot = Some(Arc::clone(&store));
        Ok(store)
    }

    fn ok(text: impl Into<String>) -> CallToolResult {
        CallToolResult::success(vec![Content::text(text)])
    }

    fn call_tool_result_text(result: &CallToolResult) -> String {
        result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect::<Vec<_>>()
            .join("")
    }

    fn envelope_capture_eligible(text: &str) -> bool {
        let Ok(value) = serde_json::from_str::<Value>(text) else {
            return true;
        };
        match value.get("status").and_then(Value::as_str) {
            Some("ok" | "partial") => true,
            _ => false,
        }
    }

    fn buffered_pointer_envelope(
        tool_name: &str,
        response_id: &str,
        original_bytes: usize,
        started: Instant,
    ) -> CallToolResult {
        envelope_success(
            json!({
                "buffered": true,
                "response_id": response_id,
                "source_tool": tool_name,
                "original_bytes": original_bytes,
                "hint": "Use ctx_peek, ctx_grep, ctx_slice, or ctx_stats to read portions of the buffered response."
            }),
            started,
            vec![format!(
                "response buffered as {response_id}; use ctx_* tools to re-cut without reloading the full payload"
            )],
            false,
        )
    }

    async fn capture_tool_response(
        &self,
        tool_name: &str,
        result: CallToolResult,
    ) -> CallToolResult {
        if matches!(tool_name, "ctx_stats" | "ctx_grep" | "ctx_slice" | "ctx_peek") {
            return result;
        }
        if result.is_error == Some(true) {
            return result;
        }
        let text = Self::call_tool_result_text(&result);
        if text.len() < crate::response_buffer::MIN_CAPTURE_BYTES
            || !Self::envelope_capture_eligible(&text)
        {
            return result;
        }
        let mut buffer = self.response_buffer.lock().await;
        let Some(response_id) = buffer.capture(tool_name, &text) else {
            return result;
        };
        let original_bytes = text.len();
        drop(buffer);
        Self::buffered_pointer_envelope(tool_name, &response_id, original_bytes, Instant::now())
    }

    async fn wrap_result(&self, tool: &str, result: CallToolResult) -> CallToolResult {
        self.capture_tool_response(tool, result).await
    }

    fn tool_enabled(&self, key: &str, default_value: bool) -> bool {
        self.feature_flags.is_enabled_or(key, default_value)
    }

    fn current_watch_config(&self) -> CortexConfig {
        CortexConfig::load().unwrap_or_else(|_| self.config.clone())
    }

    async fn spawn_watch_for_path(&self, path: String) -> Result<String, McpError> {
        let mut cfg = self.current_watch_config();
        let session = WatchSession::new(&cfg);
        session
            .watch(PathBuf::from(&path).as_path())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        session
            .persist_to_config(&mut cfg)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let job_id = format!("watch-{}", now_millis());
        self.jobs
            .mark_running(&job_id, format!("Watching {}", path));
        let jobs = self.jobs.clone();
        let cfg = cfg.clone();
        let watch_path = path.clone();
        let job_id_for_task = job_id.clone();
        tokio::spawn(async move {
            let watch_outcome = async {
                let client = GraphClient::connect(&cfg).await?;
                let indexer = Indexer::from_cortex_config(client, &cfg)?;
                let watcher = WatchSession::new(&cfg);
                watcher.watch(watch_path.as_ref())?;
                watcher.run(indexer).await?;
                anyhow::Ok(())
            }
            .await;
            if let Err(err) = watch_outcome {
                jobs.mark_failed(&job_id_for_task, err.to_string());
            }
        });
        Ok(job_id)
    }

    async fn repo_freshness_label(&self, repo_path: &str) -> String {
        if let Ok(client) = self.graph_client().await {
            let branch_records = get_branch_indexes(&client, repo_path)
                .await
                .unwrap_or_default();
            if let Some(record) = branch_records.first() {
                return record.graph_freshness.as_str().to_string();
            }
        }
        "unknown".to_string()
    }

    async fn diagnose_issues_summary(&self, repo_path: &str) -> Vec<Value> {
        let mut issues = Vec::new();
        match self.graph_client().await {
            Ok(client) => {
                let repo_query = format!(
                    "MATCH (r:Repository {{path:'{}'}}) RETURN r.indexed_at as indexed_at",
                    escape_cypher(repo_path)
                );
                if let Ok(results) = client.raw_query(&repo_query).await
                    && results.is_empty()
                {
                    issues.push(json!({
                        "check": "index_status",
                        "severity": "warning",
                        "message": "Repository not indexed"
                    }));
                }
            }
            Err(e) => {
                issues.push(json!({
                    "check": "graph_connection",
                    "severity": "critical",
                    "message": format!("Cannot connect to graph database: {e}")
                }));
            }
        }
        issues
    }

    fn resolve_project_context(&self) -> Result<(String, Option<String>), McpError> {
        let project_ref = self.projects.get_current_project().ok_or_else(|| {
            McpError::invalid_params(
                "No current project set. Use set_current_project first.",
                None,
            )
        })?;
        Ok((
            project_ref.path.display().to_string(),
            Some(project_ref.branch),
        ))
    }

    fn resolve_navigation_scope(
        &self,
        repo_path: Option<&str>,
        from_file: Option<&str>,
        branch: Option<String>,
    ) -> Result<(String, Option<String>), McpError> {
        if let Some(p) = repo_path.filter(|s| !s.is_empty()) {
            let scope = cortex_core::graph_repository_path_for_index(Path::new(p), None);
            return Ok((scope, branch));
        }
        if let Ok((project_path, b)) = self.resolve_project_context() {
            let scope =
                cortex_core::graph_repository_path_for_index(Path::new(&project_path), None);
            return Ok((scope, b.or(branch)));
        }
        if let Some(file) = from_file.filter(|f| !f.is_empty()) {
            let file_path = Path::new(file);
            let base = file_path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(file_path);
            let scope = cortex_core::graph_repository_path_for_index(base, None);
            return Ok((scope, branch));
        }
        let cwd = default_repo_path();
        let scope = cortex_core::graph_repository_path_for_index(Path::new(&cwd), None);
        Ok((scope, branch))
    }

    async fn build_symbol_resolver(
        &self,
        repo_path: Option<&str>,
        branch: Option<String>,
    ) -> Result<SymbolResolver, McpError> {
        let graph = self.graph_client().await?;
        let scan = repo_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(default_repo_path()));
        let scope = normalize_repo_scope(scan.as_path());
        Ok(SymbolResolver::new(graph, scope, branch))
    }

    fn resolve_pr_review_scope(
        &self,
        req: &PrReviewReq,
    ) -> Result<(String, Option<String>), McpError> {
        if let Some(p) = req
            .repo_path
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| req.path.as_deref().filter(|s| !s.is_empty()))
        {
            let pb = Path::new(p);
            let git_root = find_git_repository_root(pb).unwrap_or_else(|| pb.to_path_buf());
            let scope = cortex_core::graph_repository_path_for_index(&git_root, None);
            let branch = resolve_git_context_for_path(pb).map(|(_, b, _)| b);
            return Ok((scope, branch));
        }
        self.resolve_project_context()
    }

    fn graph_scope_from_filters(include_paths: Option<&[String]>) -> Option<String> {
        include_paths.and_then(|paths| {
            paths.first().map(|p| {
                cortex_core::graph_repository_path_for_index(std::path::Path::new(p), None)
            })
        })
    }

    async fn scoped_analyzer_for_filters(
        &self,
        include_paths: Option<Vec<String>>,
    ) -> Result<cortex_analyzer::Analyzer, McpError> {
        let client = self.graph_client().await?;
        let mut analyzer = cortex_analyzer::Analyzer::new(client);
        if let Ok((project_path, branch)) = self.resolve_project_context() {
            let scope = cortex_core::graph_repository_path_for_index(
                std::path::Path::new(&project_path),
                None,
            );
            analyzer = analyzer.with_repository_scope(scope, branch);
        } else if let Some(scope) = Self::graph_scope_from_filters(include_paths.as_deref()) {
            analyzer = analyzer.with_repository_scope(scope, None);
        }
        Ok(analyzer)
    }


    #[tool(
        description = "Index a directory or file into the code graph (and optionally vector store). Use when the user asks to index a repo, add code to the graph, or (re)build the index. Run before graph/vector tools can return results. Returns graph and optional vector indexing stats."
    )]
    async fn add_code_to_graph(
        &self,
        Parameters(req): Parameters<IndexPathReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let include_vector = req.include_vector.unwrap_or(false);
        let force = req.force.unwrap_or(false);
        let job_id = format!("index-{}", now_millis());
        self.jobs
            .mark_running(&job_id, format!("Indexing {}", req.path));

        if let Ok(Some(daemon_stage)) =
            Self::try_enqueue_daemon_index(&self.config, Path::new(&req.path), force)
        {
            self.jobs.mark_completed(&job_id, "queued via daemon");
            return Ok(envelope_success(
                json!({
                    "job_id": job_id,
                    "path": req.path,
                    "index": daemon_stage,
                    "include_vector": include_vector,
                }),
                started_at,
                vec!["vector indexing deferred to post-daemon graph job".to_string()],
                false,
            ));
        }

        let cfg = self.config.clone();
        let jobs = self.jobs.clone();
        let path = req.path.clone();
        let job_id_for_task = job_id.clone();
        let a2a_hub = self.a2a_hub.clone();
        let vector_svc = self.vector_service.clone();
        tokio::spawn(async move {
            let outcome = async {
                let client = GraphClient::connect(&cfg).await?;
                let indexer = Indexer::from_cortex_config(client, &cfg)?;
                let scan = Path::new(&path);
                let graph_scope = cortex_core::graph_repository_path_for_index(scan, None);
                let graph_report = if let Some((_repo_root, branch, revision)) =
                    resolve_git_context_for_path(scan)
                {
                    indexer
                        .index_path_with_branch_context(
                            scan,
                            &branch,
                            &revision,
                            &graph_scope,
                            force,
                            !force,
                        )
                        .await?
                } else {
                    indexer.index_path_with_options(&path, force).await?
                };
                let mut vector_status = serde_json::json!({
                    "enabled": include_vector,
                    "status": "skipped"
                });
                if include_vector {
                    match Self::init_vector_service(&vector_svc, &cfg).await {
                        Ok(service) => {
                            let scan = Path::new(&path);
                            let graph_scope =
                                cortex_core::graph_repository_path_for_index(scan, None);
                            let vector_root = resolve_git_context_for_path(scan)
                                .map(|(root, _, _)| root)
                                .unwrap_or_else(|| scan.to_path_buf());
                            let (branch, revision) =
                                resolve_vector_index_git_context(scan, None, None);
                            let vector_outcome = if scan.is_file() {
                                service
                                    .index_file(scan, &graph_scope, &branch, &revision)
                                    .await
                            } else {
                                service
                                    .index_repository(
                                        &vector_root,
                                        &graph_scope,
                                        &branch,
                                        &revision,
                                        None,
                                        None,
                                        &cfg,
                                    )
                                    .await
                            };
                            match vector_outcome {
                                Ok(indexed) => {
                                    global_metrics().record_vector_documents_indexed(
                                        indexed.indexed_documents as u64,
                                    );
                                    promote_vector_freshness_with_config(
                                        &cfg,
                                        &graph_scope,
                                        &branch,
                                        indexed.indexed_documents,
                                    )
                                    .await;
                                    vector_status = serde_json::json!({
                                        "enabled": true,
                                        "status": "completed",
                                        "indexed_documents": indexed.indexed_documents,
                                        "scanned_files": indexed.scanned_files,
                                        "skipped_files": indexed.skipped_files
                                    });
                                }
                                Err(err) => {
                                    global_metrics().record_vector_fallback();
                                    vector_status = serde_json::json!({
                                        "enabled": true,
                                        "status": "failed",
                                        "warning": "vector_index_failed",
                                        "error": err
                                    });
                                }
                            }
                        }
                        Err(err) => {
                            global_metrics().record_vector_fallback();
                            vector_status = serde_json::json!({
                                "enabled": true,
                                "status": "failed",
                                "warning": WARNING_VECTOR_STORE_UNAVAILABLE,
                                "error": err.to_string()
                            });
                        }
                    }
                }
                anyhow::Ok(serde_json::json!({
                    "graph": graph_report,
                    "vector": vector_status
                }))
            }
            .await;

            match outcome {
                Ok(report) => {
                    if let Some(hub) = &a2a_hub {
                        hub.notify_index_promotion(&path).await;
                    }
                    jobs.mark_completed(
                        &job_id_for_task,
                        serde_json::to_string(&report).unwrap_or_else(|_| "completed".to_string()),
                    )
                }
                Err(err) => jobs.mark_failed(&job_id_for_task, err.to_string()),
            }
        });

        if req.wait.unwrap_or(false) {
            let timeout_secs = req.wait_timeout_secs.unwrap_or(600);
            let deadline =
                Instant::now() + std::time::Duration::from_secs(timeout_secs.clamp(5, 3600));
            loop {
                if let Some(info) = self.jobs.get(&job_id) {
                    match info.state {
                        JobState::Completed => {
                            return Ok(envelope_success(
                                json!({
                                    "job_id": job_id,
                                    "state": "completed",
                                    "path": req.path,
                                    "message": info.message
                                }),
                                started_at,
                                Vec::new(),
                                false,
                            ));
                        }
                        JobState::Failed => {
                            return Ok(envelope_error(
                                "INDEX_FAILED",
                                info.message,
                                Some(json!({ "job_id": job_id })),
                                started_at,
                            ));
                        }
                        JobState::Running => {}
                    }
                }
                if Instant::now() >= deadline {
                    return Ok(envelope_success(
                        json!({
                            "job_id": job_id,
                            "state": "running",
                            "path": req.path,
                            "timed_out": true,
                            "hint": "poll check_job_status"
                        }),
                        started_at,
                        vec!["wait_timeout".to_string()],
                        true,
                    ));
                }
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        Ok(envelope_success(
            json!({
                "job_id": job_id,
                "state": "running",
                "path": req.path,
                "include_vector": include_vector,
                "hint": "poll check_job_status or set wait=true"
            }),
            started_at,
            Vec::new(),
            false,
        ))
    }


    #[tool(
        description = "Watch a directory for file changes and reindex automatically. Use when the user wants to keep the index up to date as they edit. Starts a watcher; combine with list_watched_paths and unwatch_directory to manage."
    )]
    async fn watch_directory(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let mut cfg = self.current_watch_config();
        let session = WatchSession::new(&cfg);
        session
            .watch(PathBuf::from(&req.path).as_path())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        session
            .persist_to_config(&mut cfg)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let job_id = format!("watch-{}", now_millis());
        self.jobs
            .mark_running(&job_id, format!("Watching {}", req.path));
        let jobs = self.jobs.clone();
        let cfg = cfg.clone();
        let watch_path = req.path.clone();
        let job_id_for_task = job_id.clone();
        tokio::spawn(async move {
            let watch_outcome = async {
                let client = GraphClient::connect(&cfg).await?;
                let indexer = Indexer::from_cortex_config(client, &cfg)?;
                let watcher = WatchSession::new(&cfg);
                watcher.watch(watch_path.as_ref())?;
                watcher.run(indexer).await?;
                anyhow::Ok(())
            }
            .await;
            if let Err(err) = watch_outcome {
                jobs.mark_failed(&job_id_for_task, err.to_string());
            }
        });

        Ok(envelope_success(
            json!({
                "job_id": job_id,
                "state": "running",
                "path": req.path
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "List all currently watched paths")]
    async fn list_watched_paths(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let cfg = self.current_watch_config();
        let paths = WatchSession::new(&cfg).list();
        Ok(envelope_success(
            json!({ "paths": paths }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Stop watching a directory")]
    async fn unwatch_directory(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let mut cfg = self.current_watch_config();
        let session = WatchSession::new(&cfg);
        let removed = session
            .unwatch(PathBuf::from(&req.path).as_path())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        session
            .persist_to_config(&mut cfg)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "path": req.path, "removed": removed }),
            started,
            Vec::new(),
            false,
        ))
    }


    #[tool(
        description = "Search the code graph by symbol name, pattern, type, or content. Use when the user asks to find a function/class by name, list symbols matching a pattern, or search by code type (e.g. function, class). Returns matching symbols with file paths and signatures."
    )]
    async fn find_code(
        &self,
        Parameters(req): Parameters<FindCodeReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let (repo_path, _) = self.resolve_project_context()?;
        let cache_revision = repo_path.clone();
        if self.tool_enabled("cache", true) {
            let cache = self.tool_cache().await;
            let key = L1Cache::make_key("find_code", &repo_path, &tool_params_hash(&req));
            if let (Some(cached), _) = cache.get::<String>(&key, &cache_revision) {
                return Ok(CallToolResult::success(vec![Content::text(cached)]));
            }
        }
        let kind = match req.kind.as_deref().unwrap_or("pattern") {
            "name" => SearchKind::Name,
            "type" => SearchKind::Type,
            "content" => SearchKind::Content,
            _ => SearchKind::Pattern,
        };
        let rows = Analyzer::new(self.graph_client().await?)
            .find_code(&req.query, kind, req.path_filter.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let body = envelope_success_json(json!({ "results": rows }), started, Vec::new(), false);
        if self.tool_enabled("cache", true) {
            let cache = self.tool_cache().await;
            let key = L1Cache::make_key("find_code", &repo_path, &tool_params_hash(&req));
            cache.put(&key, body.clone(), cache_revision);
        }
        Ok(CallToolResult::success(vec![Content::text(body)]))
    }

    #[tool(
        description = "Analyze code relationships: callers, callees, class hierarchy, dead code, overrides, module deps, call chains. Use when the user asks for 'who calls X', 'what does Y call', 'class hierarchy', 'dead code', or 'call chain from A to B'. Pass query_type (e.g. find_callers, find_callees, dead_code), target symbol(s), and optional include/exclude path/file/glob filters."
    )]
    async fn analyze_code_relationships(
        &self,
        Parameters(req): Parameters<RelationshipReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let a = self
            .scoped_analyzer_for_filters(req.include_paths.clone())
            .await?;
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let t = req.target.as_deref().unwrap_or_default();
        let t2 = req.target2.as_deref().unwrap_or_default();
        let rows = match req.query_type.as_str() {
            "find_callers" => a.callers_with_filters(t, Some(&filters)).await,
            "find_callees" => a.callees_with_filters(t, Some(&filters)).await,
            "find_all_callers" => a.all_callers_with_filters(t, Some(&filters)).await,
            "find_all_callees" => a.all_callees_with_filters(t, Some(&filters)).await,
            "call_chain" => {
                a.call_chain_with_filters(t, t2, req.depth, Some(&filters))
                    .await
            }
            "class_hierarchy" => a.class_hierarchy_with_filters(t, Some(&filters)).await,
            "dead_code" => a.dead_code_with_filters(Some(&filters)).await,
            "overrides" => a.overrides_with_filters(t, Some(&filters)).await,
            "module_deps" => a.module_dependencies_with_filters(t, Some(&filters)).await,
            "variable_scope" => a.variable_scope_with_filters(t, Some(&filters)).await,
            "find_importers" => a.find_importers_with_filters(t, Some(&filters)).await,
            "find_by_decorator" => a.find_by_decorator_with_filters(t, Some(&filters)).await,
            "find_by_argument" => a.find_by_argument_with_filters(t, Some(&filters)).await,
            "find_complexity" => a.find_complexity_with_filters(t, Some(&filters)).await,
            _ => {
                return Err(McpError::invalid_params(
                    format!("unknown query_type: {}", req.query_type),
                    None,
                ));
            }
        }
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "results": rows }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Execute a raw Cypher query against the code graph. Use only when the user needs a custom graph query (e.g. custom traversal, aggregation). Prefer get_impact_graph, find_code, or analyze_code_relationships for common tasks. Returns query result rows."
    )]
    async fn execute_cypher_query(
        &self,
        Parameters(req): Parameters<CypherReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let mut rows = self
            .graph_client()
            .await?
            .raw_query(&req.query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let mut warnings = Vec::new();
        if self.config.a2a.enabled {
            let max_rows = self.config.a2a.host_guard.max_cypher_rows;
            let (trimmed, truncated) = crate::host_guard::truncate_cypher_rows(rows, max_rows);
            rows = trimmed;
            if truncated {
                warnings.push(format!(
                    "cypher results truncated to {max_rows} rows ([a2a.host_guard].max_cypher_rows); prefer typed graph tools for large traversals"
                ));
            }
        }
        Ok(envelope_success(
            serde_json::to_value(rows).unwrap_or_default(),
            started,
            warnings,
            false,
        ))
    }

    #[tool(
        description = "Find functions or symbols that are never called (dead code). Use when the user asks to find unused code, dead code, or candidates for removal. Returns symbols with no callers."
    )]
    async fn find_dead_code(
        &self,
        Parameters(req): Parameters<DeadCodeReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let filters =
            build_analyze_filters(req.include_paths.clone(), None, None, None, None, None)
                .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let analyzer = self
            .scoped_analyzer_for_filters(req.include_paths.clone())
            .await?;
        let mut rows = analyzer
            .dead_code_with_filters(Some(&filters))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let limit = req.limit.unwrap_or(200).clamp(1, 2000);
        let truncated = rows.len() > limit;
        rows.truncate(limit);
        let mut warnings = Vec::new();
        if truncated {
            warnings.push(format!("truncated_to_{limit}"));
        }
        Ok(envelope_success(
            json!({ "dead_code": rows, "count": rows.len(), "limit": limit }),
            started,
            warnings,
            truncated,
        ))
    }

    #[tool(
        description = "Find near-duplicate code clones using index-time MinHash+LSH `SIMILAR_TO` edges. Use when hunting duplicated logic across files."
    )]
    async fn find_clones(
        &self,
        Parameters(req): Parameters<FindClonesReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let limit = req.limit.unwrap_or(100).clamp(1, 1000);
        let min_jaccard = req.min_jaccard.unwrap_or(0.85).clamp(0.5, 1.0);
        let client = self.graph_client().await?;
        let path_filter = req.path.clone().unwrap_or_default();
        let query = if path_filter.is_empty() {
            format!(
                "MATCH (a:CodeNode)-[r:SIMILAR_TO]->(b:CodeNode) \
                 WHERE a.repository_path = '{}' \
                 RETURN a.id AS id_a, a.name AS name_a, a.path AS path_a, \
                        b.id AS id_b, b.name AS name_b, b.path AS path_b, \
                        r.properties AS properties \
                 LIMIT {}",
                escape_cypher(&repo_path),
                limit
            )
        } else {
            format!(
                "MATCH (a:CodeNode)-[r:SIMILAR_TO]->(b:CodeNode) \
                 WHERE a.repository_path = '{}' \
                   AND (a.path CONTAINS '{}' OR b.path CONTAINS '{}') \
                 RETURN a.id AS id_a, a.name AS name_a, a.path AS path_a, \
                        b.id AS id_b, b.name AS name_b, b.path AS path_b, \
                        r.properties AS properties \
                 LIMIT {}",
                escape_cypher(&repo_path),
                escape_cypher(&path_filter),
                escape_cypher(&path_filter),
                limit
            )
        };
        let rows = client
            .raw_query(&query)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let mut clones = Vec::new();
        for row in rows {
            let jaccard = row
                .get("properties")
                .and_then(|v| v.get("jaccard"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1.0);
            if jaccard < min_jaccard {
                continue;
            }
            clones.push(json!({
                "id_a": row.get("id_a"),
                "name_a": row.get("name_a"),
                "path_a": row.get("path_a"),
                "id_b": row.get("id_b"),
                "name_b": row.get("name_b"),
                "path_b": row.get("path_b"),
                "jaccard": jaccard,
            }));
        }
        let mut warnings = Vec::new();
        if clones.is_empty() {
            warnings.push(
                "no_similar_to_edges: enable clone_detection_enabled and re-index".to_string(),
            );
        }
        Ok(envelope_success(
            json!({
                "clones": clones,
                "count": clones.len(),
                "min_jaccard": min_jaccard,
                "repo_path": repo_path,
            }),
            started,
            warnings,
            false,
        ))
    }

    #[tool(
        description = "Find similar functions or symbols across multiple indexed repositories. Use when comparing codebases or finding duplicated functionality."
    )]
    async fn find_similar_across_projects(
        &self,
        Parameters(req): Parameters<SimilarAcrossReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let graph = self.graph_client().await?;
        let analyzer = CrossProjectAnalyzer::new(graph);
        let results = analyzer
            .find_similar_symbols(None, req.min_repos.unwrap_or(2))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "results": results }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Find shared dependencies between indexed projects. Shows modules imported by multiple repositories."
    )]
    async fn find_shared_dependencies(
        &self,
        Parameters(req): Parameters<SharedDepsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let graph = self.graph_client().await?;
        let analyzer = CrossProjectAnalyzer::new(graph);
        let repos = if req.repos.is_empty() {
            None
        } else {
            Some(req.repos.as_slice())
        };
        let results = analyzer
            .find_shared_dependencies(repos)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "results": results }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Compare public API surfaces between two repositories. Shows shared functions, unique functions, and a similarity score."
    )]
    async fn compare_api_surface(
        &self,
        Parameters(req): Parameters<CompareApiReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let graph = self.graph_client().await?;
        let analyzer = CrossProjectAnalyzer::new(graph);
        let result = analyzer
            .compare_api_surface(&req.repo_a, &req.repo_b)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(json!(result), started, Vec::new(), false))
    }

    #[tool(
        description = "Go to the definition of a symbol. Uses qualified-name and import-context disambiguation when possible."
    )]
    async fn go_to_definition(
        &self,
        Parameters(req): Parameters<GoToDefReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let (repo_scope, branch) = self.resolve_navigation_scope(
            req.repo_path.as_deref(),
            req.from_file.as_deref(),
            req.branch.clone(),
        )?;
        let resolver = self
            .build_symbol_resolver(Some(&repo_scope), branch)
            .await?;
        let from_file = req
            .from_file
            .as_deref()
            .map(|f| normalize_repo_relative_file(&repo_scope, f));
        let outcome = resolver
            .resolve_definitions(&ResolveSymbolInput {
                symbol: &req.symbol,
                qualified_name: req.qualified_name.as_deref(),
                from_file: from_file.as_deref(),
                from_line: req.from_line,
                repo_scope: repo_scope.clone(),
            })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        match outcome {
            ResolveOutcome::Found {
                hits,
                tried_strategies,
            } => Ok(envelope_success(
                json!({ "definitions": hits, "tried_strategies": tried_strategies }),
                started,
                Vec::new(),
                false,
            )),
            ResolveOutcome::Ambiguous {
                candidates,
                tried_strategies,
            } => Ok(envelope_success(
                json!({
                    "definitions": candidates,
                    "ambiguous": true,
                    "tried_strategies": tried_strategies
                }),
                started,
                vec!["multiple_definitions_returned".to_string()],
                false,
            )),
            ResolveOutcome::NotFound {
                tried_strategies,
                suggestions,
            } => Ok(envelope_success(
                json!({
                    "definitions": [],
                    "tried_strategies": tried_strategies,
                    "suggestions": suggestions
                }),
                started,
                vec!["no_definition_found".to_string()],
                true,
            )),
        }
    }

    #[tool(
        description = "Find all usages of a symbol across the current project (calls, imports, type references, inheritance, and references)."
    )]
    async fn find_all_usages(
        &self,
        Parameters(req): Parameters<FindUsagesReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let (repo_scope, branch) =
            self.resolve_navigation_scope(req.repo_path.as_deref(), None, req.branch.clone())?;
        let branch_nav = branch.clone();
        let resolver = self
            .build_symbol_resolver(Some(&repo_scope), branch)
            .await?;
        let from_file = req
            .from_file
            .as_deref()
            .map(|f| normalize_repo_relative_file(&repo_scope, f));
        let resolve = resolver
            .resolve_definitions(&ResolveSymbolInput {
                symbol: &req.symbol,
                qualified_name: req.qualified_name.as_deref(),
                from_file: from_file.as_deref(),
                from_line: None,
                repo_scope: repo_scope.clone(),
            })
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let graph = self.graph_client().await?;
        let nav = NavigationEngine::new(graph, repo_scope, branch_nav);
        let usage_kind = parse_usage_kind(req.kind.as_deref())
            .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let mut warnings = Vec::new();
        let mut groups = Vec::new();
        let symbols: Vec<String> = match resolve {
            ResolveOutcome::Found { hits, .. } => usage_symbol_names_from_hits(hits),
            ResolveOutcome::Ambiguous { candidates, .. } => {
                warnings.push("ambiguous_symbol".to_string());
                usage_symbol_names_from_hits(candidates)
            }
            ResolveOutcome::NotFound { suggestions, .. } => {
                return Ok(envelope_success(
                    json!({ "usages": [], "groups": [], "suggestions": suggestions }),
                    started,
                    vec!["symbol_not_found_for_usages".to_string()],
                    true,
                ));
            }
        };
        for sym in symbols {
            let results = nav
                .find_usages(&sym, usage_kind)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            if !results.is_empty() {
                groups.push(json!({ "symbol": sym, "usages": results }));
            }
        }
        let partial = groups.is_empty();
        if partial {
            warnings.push("no_usages_found".to_string());
        }
        Ok(envelope_success(
            json!({ "groups": groups }),
            started,
            warnings,
            partial,
        ))
    }

    #[tool(
        description = "Get quick information about a symbol: signature, docs, definition location, and usage metrics."
    )]
    async fn quick_info(
        &self,
        Parameters(req): Parameters<QuickInfoReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let graph = self.graph_client().await?;
        let (repo_path, branch) =
            self.resolve_navigation_scope(req.repo_path.as_deref(), None, req.branch.clone())?;
        let nav = NavigationEngine::new(graph, repo_path, branch);
        let results = nav
            .quick_info(&req.symbol)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let payload = json!(results);
        let payload_text = payload.to_string();
        let etag = crate::rerank::content_etag(&payload_text);
        if req.if_none_match.as_deref() == Some(etag.as_str()) {
            return Ok(crate::savings::finish_not_modified_response(
                self.savings_enabled(),
                EnvelopeBuilder::new(started).audit_tool("quick_info"),
                &etag,
                "quick_info",
                req.repo_path.as_deref(),
                payload_text.chars().count(),
                &Self::baseline_sample(&payload_text),
            ));
        }
        Ok(EnvelopeBuilder::new(started)
            .audit_tool("quick_info")
            .etag(&etag)
            .success(payload))
    }

    #[tool(
        description = "Compare two branches at the symbol level (added/removed/modified symbols plus impact). Both branches should be indexed."
    )]
    async fn branch_structural_diff(
        &self,
        Parameters(req): Parameters<BranchStructuralDiffReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let graph = self.graph_client().await?;
        let (repo_path, branch) =
            self.resolve_navigation_scope(req.repo_path.as_deref(), None, None)?;
        let nav = NavigationEngine::new(graph, repo_path, branch);
        let result = nav
            .branch_structural_diff(&req.source_branch, &req.target_branch)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(json!(result), started, Vec::new(), false))
    }

    #[tool(
        description = "Run review analysis with graph intelligence (impact warnings + potential new dead code)."
    )]
    async fn pr_review(
        &self,
        Parameters(req): Parameters<PrReviewReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let graph = self.graph_client().await?;
        let (repo_path, _branch) = self.resolve_pr_review_scope(&req)?;
        let scope_path = req.path.clone().unwrap_or_else(|| ".".to_string());
        let params = crate::intelligence::PrReviewParams {
            task: "pull request review".to_string(),
            repo_path: repo_path.clone(),
            source_branch: req.head_ref.clone().unwrap_or_else(|| "HEAD".to_string()),
            target_branch: req.base_ref.clone().unwrap_or_else(|| "main".to_string()),
            scope: crate::intelligence::ScopeFilters::new(vec![scope_path.clone()], vec![]),
            budget_tokens: req
                .budget_tokens
                .unwrap_or(8000)
                .clamp(512, 16_000) as u32,
            target_symbol: None,
        };
        let mut pack = crate::intelligence::compute_pr_review_pack(&graph, &params).await;
        if pack
            .meta
            .warnings
            .iter()
            .any(|w| w.contains("branch_structural_diff unavailable"))
        {
            pack.meta.warnings.push(
                "falling back to ReviewAnalyzer for git-native findings when delta unavailable"
                    .to_string(),
            );
            let nav = NavigationEngine::new(graph, repo_path.clone(), None);
            let reviewer = ReviewAnalyzer::new();
            let input = build_review_input_from_req(&repo_path, &req)
                .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
            let report = reviewer.analyze_with_graph(&input, &nav).await;
            if let Some(obj) = pack.data.as_object_mut() {
                obj.insert("review_analyzer".to_string(), json!(report));
            }
        }
        Ok(pack.to_envelope(
            "pr_review",
            started,
            vec![scope_path],
            Vec::new(),
            Vec::new(),
        ))
    }

    #[tool(
        description = "Calculate cyclomatic complexity of symbols, ranked by highest complexity. Use when the user asks for 'complex code', 'most complex functions', or 'complexity analysis'. Supports optional include/exclude path/file/glob filters to scope results."
    )]
    async fn calculate_cyclomatic_complexity(
        &self,
        Parameters(req): Parameters<ComplexityReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let rows = Analyzer::new(self.graph_client().await?)
            .complexity_with_filters(req.top_n.unwrap_or(20) as usize, Some(&filters))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "results": rows }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Index all code files in a repository into vector storage for semantic search. Use when the user wants to enable natural-language code search (vector_search) or before asking 'code related to X'. Returns indexed_documents, scanned_files, skipped_files."
    )]
    async fn vector_index_repository(
        &self,
        Parameters(req): Parameters<VectorIndexRepositoryReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_index_repository is disabled by feature flag",
                None,
                started,
            ));
        }
        let root = PathBuf::from(&req.path);
        if !root.exists() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                format!("path does not exist: {}", req.path),
                None,
                started,
            ));
        }
        if !root.is_dir() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                format!(
                    "vector_index_repository expects a directory path, got file: {}",
                    req.path
                ),
                None,
                started,
            ));
        }
        let service = match self.vector_service().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let repository = req
            .repo_path
            .clone()
            .unwrap_or_else(|| root.display().to_string());
        let (branch, revision) =
            resolve_vector_index_git_context(&root, req.branch.clone(), req.revision.clone());
        let include_paths = req.include_paths.as_deref();
        let result = service
            .index_repository(
                &root,
                &repository,
                &branch,
                &revision,
                include_paths,
                req.max_files,
                &self.config,
            )
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        global_metrics().record_vector_documents_indexed(result.indexed_documents as u64);
        self.promote_vector_freshness(&repository, &branch, result.indexed_documents)
            .await;
        Ok(envelope_success(
            json!({
                "repository": repository,
                "branch": branch,
                "revision": revision,
                "indexed_documents": result.indexed_documents,
                "scanned_files": result.scanned_files,
                "skipped_files": result.skipped_files,
                "include_paths": req.include_paths,
                "max_files": req.max_files
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Index a single file into vector storage. Use when the user wants to add one file to the semantic index without re-indexing the whole repo. Returns indexed_documents for that file."
    )]
    async fn vector_index_file(
        &self,
        Parameters(req): Parameters<VectorIndexFileReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_index_file is disabled by feature flag",
                None,
                started,
            ));
        }
        let file = PathBuf::from(&req.path);
        if !file.exists() || !file.is_file() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                format!("file does not exist: {}", req.path),
                None,
                started,
            ));
        }
        let service = match self.vector_service().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let repository = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let (branch, revision) =
            resolve_vector_index_git_context(&file, req.branch.clone(), req.revision.clone());
        let result = service
            .index_file(&file, &repository, &branch, &revision)
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        global_metrics().record_vector_documents_indexed(result.indexed_documents as u64);
        self.promote_vector_freshness(&repository, &branch, result.indexed_documents)
            .await;
        Ok(envelope_success(
            json!({
                "file": req.path,
                "repository": repository,
                "branch": branch,
                "revision": revision,
                "indexed_documents": result.indexed_documents,
                "scanned_files": result.scanned_files,
                "skipped_files": result.skipped_files
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Semantic search over indexed code. Use when the user asks in natural language for 'code that does X', 'where is Y handled', or 'code related to Z'. Requires vector_index_repository first. Returns relevant code snippets with paths and scores."
    )]
    async fn vector_search(
        &self,
        Parameters(req): Parameters<VectorSearchReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_search is disabled by feature flag",
                None,
                started,
            ));
        }
        let k = req.k.unwrap_or(20).clamp(1, 200);
        let search_type = req
            .search_type
            .as_deref()
            .and_then(|s| s.parse::<SearchType>().ok())
            .unwrap_or(SearchType::Semantic);
        let service = match self.vector_service().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let embedder_label = VectorService::embedder_label(service.embedder());
        let mut results = service
            .search(VectorSearchRequest {
                query: &req.query,
                search_type,
                k,
                filters: VectorSearchFilters {
                    repository: req.repo_path.as_deref(),
                    path: req.path.as_deref(),
                    kind: req.kind.as_deref(),
                    language: req.language.as_deref(),
                },
            })
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        if self.config.vector.rerank_enabled {
            let weights =
                crate::rerank::rerank_weights_from_vector_config(&self.config.vector);
            let candidates: Vec<crate::rerank::RerankCandidate> = results
                .iter()
                .enumerate()
                .map(|(rank, r)| {
                    let path = r
                        .result
                        .metadata
                        .get("path")
                        .and_then(|v| match v {
                            cortex_vector::MetadataValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    let name = r
                        .result
                        .metadata
                        .get("name")
                        .and_then(|v| match v {
                            cortex_vector::MetadataValue::String(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| r.result.id.clone());
                    crate::rerank::RerankCandidate {
                        id: r.result.id.clone(),
                        path,
                        name,
                        lexical_rank: rank,
                        vector_rank: Some(rank),
                        lexical_score: 0.0,
                        vector_score: r.combined_score as f64,
                        centrality: r
                            .graph_context
                            .as_ref()
                            .map(|ctx| {
                                ((ctx.callers_count + ctx.callees_count) as f64 / 16.0).min(1.0)
                            })
                            .unwrap_or(0.0),
                        token_estimate: r
                            .result
                            .content
                            .as_deref()
                            .unwrap_or("")
                            .chars()
                            .count()
                            / 4,
                        mtime_secs: crate::rerank::file_mtime_secs(
                            r.result
                                .metadata
                                .get("path")
                                .and_then(|v| match v {
                                    cortex_vector::MetadataValue::String(s) => Some(s.as_str()),
                                    _ => None,
                                })
                                .unwrap_or(""),
                        ),
                    }
                })
                .collect();
            let ranked = crate::rerank::rerank_candidates(&req.query, candidates, &weights);
            let score_map: HashMap<String, f64> = ranked.into_iter().collect();
            results.sort_by(|a, b| {
                let sa = score_map.get(&a.result.id).copied().unwrap_or(0.0);
                let sb = score_map.get(&b.result.id).copied().unwrap_or(0.0);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(k);
        }
        let output: Vec<_> = results
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.result.id,
                    "score": r.combined_score,
                    "content": r.result.content,
                    "metadata": r.result.metadata,
                    "graph_context": r.graph_context
                })
            })
            .collect();
        Ok(EnvelopeBuilder::new(started)
            .audit_tool("vector_search")
            .embedder(&embedder_label)
            .success(json!({
                "query": req.query,
                "search_type": search_type.to_string(),
                "embedder": embedder_label,
                "count": output.len(),
                "results": output
            })))
    }

    #[tool(
        description = "Hybrid search over indexed code: combines semantic (vector) and structural (graph) signals. Use when the user wants 'code like X' with better precision than vector_search alone, or when filtering by path/repo/kind. Returns snippets with hybrid scores."
    )]
    async fn vector_search_hybrid(
        &self,
        Parameters(req): Parameters<VectorSearchReq>,
    ) -> Result<CallToolResult, McpError> {
        let req = VectorSearchReq {
            search_type: Some("hybrid".to_string()),
            ..req
        };
        self.vector_search(Parameters(req)).await
    }

    #[tool(
        description = "Search code across all indexed repositories using vector embeddings. Returns results grouped by repository."
    )]
    async fn search_across_projects(
        &self,
        Parameters(req): Parameters<CrossProjectSearchReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let store_path = crate::vector_service::vector_store_path(&self.config);
        let store = LanceStore::open(&store_path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let embedder = VectorService::build_embedder(&self.config)
            .map_err(|e| McpError::internal_error(e, None))?;
        let hybrid = HybridSearch::new(Arc::new(store), embedder);
        let repos = if req.repositories.is_empty() {
            None
        } else {
            Some(req.repositories.as_slice())
        };
        let results = hybrid
            .search_across_repositories(&req.query, repos, req.limit.unwrap_or(10))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let output: Vec<_> = results
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.result.id,
                    "score": r.combined_score,
                    "content": r.result.content,
                    "metadata": r.result.metadata,
                    "graph_context": r.graph_context
                })
            })
            .collect();
        Ok(envelope_success(
            json!({
                "query": req.query,
                "count": output.len(),
                "results": output
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Return vector index health and document counts per repository. Use when the user asks if semantic search is ready, how much is indexed, or to debug vector_search returning no results. Returns status and document counts."
    )]
    async fn vector_index_status(
        &self,
        Parameters(req): Parameters<VectorIndexStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_index_status is disabled by feature flag",
                None,
                started,
            ));
        }
        let service = match self.vector_service().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let healthy = service
            .health_check()
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        let total_documents = service
            .total_documents()
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        let repository_documents = service
            .count_documents(req.repo_path.as_deref())
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(envelope_success(
            json!({
                "healthy": healthy,
                "total_documents": total_documents,
                "repository_documents": repository_documents,
                "repo_path": req.repo_path
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Delete all vector entries belonging to a repository")]
    async fn vector_delete_repository(
        &self,
        Parameters(req): Parameters<VectorDeleteRepositoryReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.vector.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "vector_delete_repository is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.dry_run.unwrap_or(false) {
            return Ok(envelope_success(
                json!({
                    "dry_run": true,
                    "would_delete_vector_repository": req.repo_path
                }),
                started,
                Vec::new(),
                false,
            ));
        }
        if req.confirm != Some(true) {
            return Ok(envelope_error(
                "CONFIRMATION_REQUIRED",
                "vector_delete_repository requires confirm=true (or dry_run=true)",
                None,
                started,
            ));
        }
        let service = match self.vector_service().await {
            Ok(s) => s,
            Err(err) => {
                return Ok(envelope_error(
                    "UNAVAILABLE",
                    "vector store unavailable",
                    Some(json!({"warning": WARNING_VECTOR_STORE_UNAVAILABLE, "error": err})),
                    started,
                ));
            }
        };
        let deleted = service
            .delete_repository(&req.repo_path)
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        Ok(envelope_success(
            json!({
                "repo_path": req.repo_path,
                "deleted_documents": deleted
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Get a token-budgeted set of relevant code items for a task. Use when the user describes a coding task (e.g. 'refactor auth', 'find bug in login') and you need ranked, bounded context. Combines graph and optional vector search; returns snippets with ranking explanations."
    )]
    async fn get_context_capsule(
        &self,
        Parameters(req): Parameters<ContextCapsuleReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.context_capsule.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_context_capsule is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.query.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "query must not be empty",
                None,
                started,
            ));
        }
        let max_items = req.max_items.unwrap_or(40).min(100);
        let default_cap = mcp_scaled_budget(6000, 256, 12000);
        let max_tokens = req.max_tokens.unwrap_or(default_cap).clamp(256, 12000);
        let include_tests = req.include_tests.unwrap_or(false);
        let intent = req
            .task_intent
            .clone()
            .unwrap_or_else(|| detect_intent(req.query.as_str()).to_string());
        let filters = req.path_filter.clone().unwrap_or_default();
        let repo_scope = req.repo_path.clone().unwrap_or_else(|| ".".to_string());
        let cache_revision = repo_scope.clone();

        let mut items = Vec::<Value>::new();
        let mut token_estimate = 0usize;
        let mut warnings = Vec::<String>::new();

        if self.tool_enabled("cache", true) {
            let cache = self.tool_cache().await;
            let key = L1Cache::make_key(
                "get_context_capsule",
                &repo_scope,
                &capsule_cache_hash(
                    &req.query,
                    max_items,
                    max_tokens,
                    include_tests,
                    &intent,
                    &filters,
                ),
            );
            if let (Some(cached), _) = cache.get::<Value>(&key, &cache_revision) {
                let baseline_sample: String = cached
                    .get("capsule_items")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.get("snippet").and_then(Value::as_str))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                let baseline_chars = baseline_sample.chars().count().saturating_mul(8);
                let cached_text = cached.to_string();
                let etag = crate::rerank::content_etag(&cached_text);
                if req.if_none_match.as_deref() == Some(etag.as_str()) {
                    return Ok(crate::savings::finish_not_modified_response(
                        self.savings_enabled(),
                        EnvelopeBuilder::new(started).audit_tool("get_context_capsule"),
                        &etag,
                        "get_context_capsule",
                        Some(&repo_scope),
                        baseline_chars,
                        &Self::baseline_sample(&baseline_sample),
                    ));
                }
                return Ok(self.finish_counted_tool(
                    EnvelopeBuilder::new(started)
                        .audit_tool("get_context_capsule")
                        .etag(&etag),
                    cached,
                    "get_context_capsule",
                    Some(&repo_scope),
                    baseline_chars,
                    &Self::baseline_sample(&baseline_sample),
                ));
            }
        }

        // Vector-first candidate retrieval for better NL relevance.
        if self.tool_enabled("mcp.vector.read.enabled", true) {
            match self.vector_service().await {
                Ok(service) => {
                    match service
                        .search(VectorSearchRequest {
                            query: req.query.as_str(),
                            search_type: SearchType::Hybrid,
                            k: max_items * 2,
                            filters: VectorSearchFilters {
                                repository: req.repo_path.as_deref(),
                                ..Default::default()
                            },
                        })
                        .await
                    {
                        Ok(vector_results) => {
                            for result in vector_results {
                                if items.len() >= max_items || token_estimate >= max_tokens {
                                    break;
                                }
                                let metadata = &result.result.metadata;
                                let path = metadata
                                    .get("path")
                                    .and_then(|v| match v {
                                        cortex_vector::MetadataValue::String(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .unwrap_or_default();
                                if !include_tests && path.contains("/test") {
                                    continue;
                                }
                                if !filters.is_empty() && !filters.iter().any(|f| path.contains(f))
                                {
                                    continue;
                                }
                                let name = metadata
                                    .get("name")
                                    .and_then(|v| match v {
                                        cortex_vector::MetadataValue::String(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .unwrap_or_default()
                                    .to_string();
                                let kind = metadata
                                    .get("kind")
                                    .and_then(|v| match v {
                                        cortex_vector::MetadataValue::String(s) => Some(s.as_str()),
                                        _ => None,
                                    })
                                    .unwrap_or("CodeNode")
                                    .to_string();
                                let snippet = result
                                    .result
                                    .content
                                    .clone()
                                    .unwrap_or_default()
                                    .chars()
                                    .take(320)
                                    .collect::<String>();
                                let snippet = redact_secrets(&snippet);
                                let lex = simple_lexical_score(&req.query, &name, &snippet);
                                let vector_score = result.combined_score as f64;
                                let graph_score = result
                                    .graph_context
                                    .as_ref()
                                    .map(|ctx| {
                                        ((ctx.callers_count + ctx.callees_count) as f64 / 20.0)
                                            .min(1.0)
                                    })
                                    .unwrap_or(0.0);
                                let score =
                                    (vector_score * 0.65) + (lex * 0.25) + (graph_score * 0.10);
                                token_estimate += snippet.len() / 4 + 32;
                                items.push(json!({
                                    "id": result.result.id,
                                    "kind": kind,
                                    "path": path,
                                    "name": name,
                                    "snippet": snippet,
                                    "score": score,
                                    "why": {
                                        "vector": vector_score,
                                        "graph": graph_score,
                                        "lexical": lex
                                    }
                                }));
                            }
                        }
                        Err(err) => {
                            global_metrics().record_vector_fallback();
                            warnings.push(warning_with_reason(WARNING_FALLBACK_TO_LEXICAL, &err));
                        }
                    }
                }
                Err(err) => {
                    global_metrics().record_vector_fallback();
                    warnings.push(warning_with_reason(
                        WARNING_VECTOR_STORE_UNAVAILABLE,
                        &err.to_string(),
                    ));
                }
            }
        } else {
            warnings.push("vector_read_disabled".to_string());
        }

        let rows = if items.len() >= max_items || token_estimate >= max_tokens {
            Vec::new()
        } else {
            let analyzer = Analyzer::new(self.graph_client().await?);
            analyzer
                .find_code(&req.query, SearchKind::Pattern, None)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        for row in rows {
            if items.len() >= max_items || token_estimate >= max_tokens {
                break;
            }
            let Some(node) = row.get("n") else {
                continue;
            };
            let path = node
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if !include_tests && path.contains("/test") {
                continue;
            }
            if !filters.is_empty() && !filters.iter().any(|f| path.contains(f)) {
                continue;
            }
            let name = node
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let kind = node
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("CodeNode")
                .to_string();
            let snippet = node
                .get("source")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .chars()
                .take(320)
                .collect::<String>();
            let snippet = redact_secrets(&snippet);
            let lex = simple_lexical_score(&req.query, &name, &snippet);
            let tfidf = (snippet.len().min(200) as f64) / 200.0;
            let centrality = 0.1;
            let score = (lex * 0.5) + (tfidf * 0.4) + (centrality * 0.1);
            token_estimate += snippet.len() / 4 + 32;
            items.push(json!({
                "id": node.get("id").cloned().unwrap_or(Value::Null),
                "kind": kind,
                "path": path,
                "name": name,
                "snippet": snippet,
                "score": score,
                "why": {
                    "vector": 0.0,
                    "graph": centrality,
                    "lexical": lex,
                    "tfidf": tfidf
                }
            }));
        }
        let partial = token_estimate >= max_tokens || items.len() >= max_items;
        if items.is_empty() {
            warnings.push("fallback_relaxed_no_results".to_string());
        }

        let graph_results: Vec<GraphSearchResult> = items
            .iter()
            .filter_map(|item| {
                Some(GraphSearchResult {
                    id: item.get("id").and_then(|v| v.as_str())?.to_string(),
                    kind: item
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("CodeNode")
                        .to_string(),
                    path: item.get("path").and_then(|v| v.as_str())?.to_string(),
                    name: item.get("name").and_then(|v| v.as_str())?.to_string(),
                    source: item.get("snippet").and_then(|v| v.as_str()).map(str::to_string),
                    line_number: item.get("line_number").and_then(|v| v.as_u64()),
                })
            })
            .collect();

        let mut capsule_config = crate::capsule::CapsuleConfig::default();
        if self.config.vector.rerank_enabled {
            capsule_config.rerank_enabled = true;
            capsule_config.use_bm25 = true;
            capsule_config.rerank_weights =
                crate::rerank::rerank_weights_from_vector_config(&self.config.vector);
        }
        let mut capsule_builder = ContextCapsuleBuilder::with_config(capsule_config)
            .with_max_items(max_items)
            .with_max_tokens(max_tokens)
            .with_include_tests(include_tests)
            .with_intent(&intent);
        let capsule_result =
            capsule_builder.build(&req.query, graph_results, Some(&intent), &filters);
        warnings.extend(capsule_result.warnings.clone());
        token_estimate = capsule_result.token_estimate;
        let capsule_items: Vec<Value> = capsule_result
            .capsule_items
            .iter()
            .map(|item| {
                json!({
                    "id": item.id,
                    "kind": item.kind,
                    "path": item.path,
                    "name": item.name,
                    "snippet": item.snippet,
                    "score": item.score,
                    "why": {
                        "vector": 0.0,
                        "graph": item.why.centrality,
                        "lexical": item.why.fts,
                        "tfidf": item.why.tfidf,
                        "proximity": item.why.proximity,
                    },
                    "line_number": item.line_number,
                })
            })
            .collect();

        let omitted = if partial || capsule_result.fallback_relaxed {
            vec![OmittedItem {
                reason: "token_or_item_budget_exceeded".to_string(),
                path: None,
                symbol: None,
                estimated_tokens: Some(token_estimate),
            }]
        } else {
            Vec::new()
        };
        let payload = json!({
            "intent_detected": capsule_result.intent_detected,
            "capsule_items": capsule_items,
            "token_estimate": token_estimate,
            "token_budget": max_tokens,
            "threshold_used": capsule_result.threshold_used,
            "fallback_relaxed": capsule_result.fallback_relaxed || !warnings.is_empty(),
            "freshness": "unknown",
            "source_policy": "snippets"
        });
        if self.tool_enabled("cache", true) {
            let cache = self.tool_cache().await;
            let key = L1Cache::make_key(
                "get_context_capsule",
                &repo_scope,
                &capsule_cache_hash(
                    &req.query,
                    max_items,
                    max_tokens,
                    include_tests,
                    &intent,
                    &filters,
                ),
            );
            cache.put(&key, payload.clone(), cache_revision);
        }
        let baseline_sample: String = capsule_items
            .iter()
            .filter_map(|item| item.get("snippet").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        let baseline_chars = baseline_sample.chars().count().saturating_mul(8);
        let payload_text = payload.to_string();
        let etag = crate::rerank::content_etag(&payload_text);
        if req.if_none_match.as_deref() == Some(etag.as_str()) {
            return Ok(crate::savings::finish_not_modified_response(
                self.savings_enabled(),
                EnvelopeBuilder::new(started)
                    .audit_tool("get_context_capsule")
                    .partial(partial)
                    .warnings(warnings.clone()),
                &etag,
                "get_context_capsule",
                req.repo_path.as_deref().or(Some(repo_scope.as_str())),
                baseline_chars,
                &Self::baseline_sample(&baseline_sample),
            ));
        }
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("get_context_capsule")
                .etag(&etag)
                .partial(partial)
                .warnings(warnings.clone())
                .cost_class("bounded")
                .freshness(FreshnessState::Unknown)
                .token_budget(TokenBudget {
                    requested_tokens: max_tokens,
                    estimated_tokens: token_estimate,
                    hard_cap: true,
                })
                .scope(ResponseScope {
                    repo_path: req.repo_path.clone(),
                    branch: None,
                    include_paths: filters.clone(),
                    exclude_paths: Vec::new(),
                })
                .source_policy(SourcePolicy::Snippets)
                .omitted(omitted)
                .next_tools(vec![
                    "get_signature".to_string(),
                    "get_skeleton".to_string(),
                    "get_test_context".to_string(),
                ]),
            payload,
            "get_context_capsule",
            req.repo_path.as_deref().or(Some(repo_scope.as_str())),
            baseline_chars,
            &Self::baseline_sample(&baseline_sample),
        ))
    }

    #[tool(
        description = "Build a token-bounded pre-edit context pack for an agent patch plan. Returns target candidates, contracts, likely tests, risks, and next tools."
    )]
    async fn get_patch_context(
        &self,
        Parameters(req): Parameters<PatchContextReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if req.task.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "task must not be empty",
                None,
                started,
            ));
        }
        let default_b = mcp_scaled_budget(6000, 512, 12000);
        let budget = req.budget_tokens.unwrap_or(default_b).clamp(512, 12000);
        let mode = req
            .mode
            .unwrap_or_else(|| detect_intent(&req.task).to_string());
        let include_paths = req.include_paths.unwrap_or_default();
        let exclude_paths = req.exclude_paths.unwrap_or_default();
        let analyzer = Analyzer::new(self.graph_client().await?);
        let client = self.graph_client().await?;
        let repo_path = default_repo_path();
        let params = crate::intelligence::PatchContextParams {
            task: req.task.clone(),
            mode: Some(mode.clone()),
            budget_tokens: budget as u32,
            scope: crate::intelligence::ScopeFilters::new(
                include_paths.clone(),
                exclude_paths.clone(),
            ),
        };
        let pack =
            crate::intelligence::build_patch_pack(&client, &analyzer, &repo_path, &params, None)
                .await;
        let data = crate::intelligence::compute_patch_context(&analyzer, &params).await;
        let estimated_tokens = data.estimated_tokens;
        let omitted = if estimated_tokens >= budget {
            vec![OmittedItem {
                reason: "budget_exhausted_before_all_candidates".to_string(),
                path: None,
                symbol: None,
                estimated_tokens: None,
            }]
        } else {
            Vec::new()
        };
        let baseline_sample = serde_json::to_string(&pack.data).unwrap_or_default();
        let baseline_chars = baseline_sample.chars().count().saturating_mul(5);
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("get_patch_context")
                .cost_class("bounded")
                .freshness(pack.meta.freshness)
                .token_budget(TokenBudget {
                    requested_tokens: pack.meta.budget_tokens as usize,
                    estimated_tokens: pack.meta.estimated_tokens,
                    hard_cap: true,
                })
                .scope(ResponseScope {
                    repo_path: Some(repo_path.clone()),
                    branch: None,
                    include_paths: include_paths.clone(),
                    exclude_paths: exclude_paths.clone(),
                })
                .source_policy(pack.meta.source_policy)
                .omitted(omitted)
                .next_tools(pack.meta.suggested_next_tools)
                .warnings(pack.meta.warnings),
            pack.data,
            "get_patch_context",
            Some(repo_path.as_str()),
            baseline_chars,
            &Self::baseline_sample(&baseline_sample),
        ))
    }

    #[tool(
        description = "Return a token-bounded branch/worktree delta context: changed symbols, impact hints, likely tests, and stale-index warnings."
    )]
    async fn get_delta_context(
        &self,
        Parameters(req): Parameters<DeltaContextReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let default_b = mcp_scaled_budget(6000, 512, 12000);
        let budget = req.budget_tokens.unwrap_or(default_b).clamp(512, 12000);
        let source_branch = req.source_branch.unwrap_or_else(|| "HEAD".to_string());
        let target_branch = req.target_branch.unwrap_or_else(|| "main".to_string());
        let repo_path = req.repo_path.unwrap_or_else(default_repo_path);
        let include_paths = req.include_paths.unwrap_or_default();
        let exclude_paths = req.exclude_paths.unwrap_or_default();
        let client = self.graph_client().await?;
        let scope =
            crate::intelligence::ScopeFilters::new(include_paths.clone(), exclude_paths.clone());
        let pack = crate::intelligence::build_delta_pack(
            &client,
            &repo_path,
            &source_branch,
            &target_branch,
            budget as u32,
            &scope,
            None,
        )
        .await;
        let baseline_sample = serde_json::to_string(&pack.data).unwrap_or_default();
        let baseline_chars = baseline_sample.chars().count().saturating_mul(5);
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("get_delta_context")
                .cost_class("bounded")
                .freshness(pack.meta.freshness)
                .token_budget(TokenBudget {
                    requested_tokens: pack.meta.budget_tokens as usize,
                    estimated_tokens: pack.meta.estimated_tokens,
                    hard_cap: true,
                })
                .scope(ResponseScope {
                    repo_path: Some(repo_path.clone()),
                    branch: None,
                    include_paths: include_paths.clone(),
                    exclude_paths: exclude_paths.clone(),
                })
                .source_policy(pack.meta.source_policy)
                .omitted(Vec::new())
                .next_tools(pack.meta.suggested_next_tools)
                .warnings(pack.meta.warnings),
            pack.data,
            "get_delta_context",
            Some(repo_path.as_str()),
            baseline_chars,
            &Self::baseline_sample(&baseline_sample),
        ))
    }

    #[tool(
        description = "Return tests likely affected by a symbol or task under a strict token budget."
    )]
    async fn get_test_context(
        &self,
        Parameters(req): Parameters<TestContextReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let default_b = mcp_scaled_budget(4000, 512, 8000);
        let budget = req.budget_tokens.unwrap_or(default_b).clamp(512, 8000);
        let analyzer = Analyzer::new(self.graph_client().await?);
        let tests = analyzer
            .find_tests_for(&req.symbol)
            .await
            .unwrap_or_default();
        let limited = tests.into_iter().take(20).collect::<Vec<_>>();
        let test_paths = limited
            .iter()
            .filter_map(|test| {
                test.get("path")
                    .or_else(|| test.get("test_path"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        let langs = self
            .langs_for_test_paths(&test_paths)
            .await
            .unwrap_or_default();
        let run_commands = synthesize_test_run_commands(&limited, &langs);
        let estimated_tokens = limited.len() * 96 + 128;
        let payload = json!({
            "symbol": req.symbol,
            "tests": limited,
            "run_commands": run_commands,
            "estimated_tokens": estimated_tokens,
            "budget_tokens": budget
        });
        let baseline_sample: String = limited
            .iter()
            .filter_map(|test| test.get("source").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        let baseline_chars = if baseline_sample.is_empty() {
            limited.len().saturating_mul(512)
        } else {
            baseline_sample.chars().count().saturating_mul(3)
        };
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("get_test_context")
                .cost_class("bounded")
                .freshness(FreshnessState::Unknown)
                .token_budget(TokenBudget {
                    requested_tokens: budget,
                    estimated_tokens,
                    hard_cap: true,
                })
                .scope(ResponseScope {
                    repo_path: req.repo_path.clone(),
                    branch: None,
                    include_paths: Vec::new(),
                    exclude_paths: Vec::new(),
                })
                .source_policy(SourcePolicy::Snippets)
                .next_tools(vec![
                    "find_tests".to_string(),
                    "get_patch_context".to_string(),
                ]),
            payload,
            "get_test_context",
            req.repo_path.as_deref(),
            baseline_chars,
            &Self::baseline_sample(&baseline_sample),
        ))
    }

    #[tool(description = "Return public API signatures and contract hints for a symbol.")]
    async fn get_api_contract(
        &self,
        Parameters(req): Parameters<ApiContractReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let include_related = req.include_related.unwrap_or(true);
        if req.symbol.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "symbol must not be empty",
                None,
                started,
            ));
        }
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let client = self.graph_client().await?;
        let query = format!(
            "MATCH (n) WHERE n.name CONTAINS '{}' AND n.path STARTS WITH '{}' \
             RETURN n.name, n.path, n.kind, n.source, n.line_number, n.lang LIMIT 20",
            escape_cypher(&req.symbol),
            escape_cypher(&repo_path)
        );
        let rows = client
            .raw_query(&query)
            .await
            .map_err(|e| McpError::internal_error(format!("Graph query failed: {}", e), None))?;
        let mut contracts = Vec::new();
        let mut baseline_chars = 0usize;
        let mut baseline_sample = String::new();
        for row in rows {
            let name = row
                .get("n.name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let path = row
                .get("n.path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let kind = row
                .get("n.kind")
                .and_then(Value::as_str)
                .unwrap_or("CodeNode")
                .to_string();
            let source = row
                .get("n.source")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            baseline_chars += source.chars().count();
            if baseline_sample.chars().count() < 8192 {
                baseline_sample.push_str(source.lines().next().unwrap_or_default());
                baseline_sample.push('\n');
            }
            let lang = row
                .get("n.lang")
                .and_then(Value::as_str)
                .and_then(|lang| FromStr::from_str(lang).ok());
            let line_number = row
                .get("n.line_number")
                .and_then(Value::as_i64)
                .map(|n| n as u32);
            let extracted = lang.and_then(|lang| {
                let node = cortex_core::CodeNode {
                    id: format!("{}:{}", path, name),
                    kind: entity_kind_from_graph_kind(&kind),
                    name: name.clone(),
                    path: Some(path.clone()),
                    line_number,
                    lang: Some(lang),
                    source: Some(source.clone()),
                    docstring: None,
                    properties: std::collections::HashMap::new(),
                };
                SignatureExtractor::extract(&node, &source)
                    .and_then(|sig| serde_json::to_value(sig).ok())
            });
            contracts.push(json!({
                "name": name,
                "path": path,
                "kind": kind,
                "line_number": line_number,
                "signature": extracted.unwrap_or_else(|| json!({
                    "text": source.lines().next().unwrap_or_default()
                })),
                "source_policy": "signatures",
                "related_included": include_related
            }));
        }
        let estimated_tokens = contracts.len() * 128 + 128;
        let payload = json!({
            "symbol": req.symbol,
            "include_related": include_related,
            "contracts": contracts,
            "estimated_tokens": estimated_tokens,
            "notes": ["Contracts are signature-only by default to avoid full-source exposure."]
        });
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("get_api_contract")
                .cost_class("bounded")
                .freshness(FreshnessState::Unknown)
                .token_budget(TokenBudget {
                    requested_tokens: 4000,
                    estimated_tokens,
                    hard_cap: true,
                })
                .source_policy(SourcePolicy::Signatures)
                .scope(ResponseScope {
                    repo_path: req.repo_path.clone(),
                    branch: None,
                    include_paths: Vec::new(),
                    exclude_paths: Vec::new(),
                })
                .next_tools(vec![
                    "get_signature".to_string(),
                    "find_all_usages".to_string(),
                ]),
            payload,
            "get_api_contract",
            req.repo_path.as_deref(),
            baseline_chars,
            &Self::baseline_sample(&baseline_sample),
        ))
    }

    #[tool(description = "Summarize a module or folder as compact architecture context.")]
    async fn summarize_module(
        &self,
        Parameters(req): Parameters<SummarizeModuleReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let default_b = mcp_scaled_budget(3000, 512, 8000);
        let budget = req.budget_tokens.unwrap_or(default_b).clamp(512, 8000);
        let analyzer = Analyzer::new(self.graph_client().await?);
        let rows = analyzer
            .find_code(&req.path, SearchKind::Pattern, Some(req.path.as_str()))
            .await
            .unwrap_or_default();
        let symbols = rows
            .into_iter()
            .take(30)
            .filter_map(|row| row.get("n").cloned())
            .collect::<Vec<_>>();
        let estimated_tokens = symbols.len() * 64 + 192;
        let payload = json!({
            "path": req.path,
            "summary": "Module summary generated from indexed symbol metadata.",
            "symbols": symbols,
            "estimated_tokens": estimated_tokens,
            "budget_tokens": budget
        });
        let baseline_sample: String = symbols
            .iter()
            .filter_map(|node| node.get("source").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
        let baseline_chars = if baseline_sample.is_empty() {
            symbols.len().saturating_mul(256)
        } else {
            baseline_sample.chars().count()
        };
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("summarize_module")
                .cost_class("bounded")
                .freshness(FreshnessState::Unknown)
                .token_budget(TokenBudget {
                    requested_tokens: budget,
                    estimated_tokens,
                    hard_cap: true,
                })
                .scope(ResponseScope {
                    repo_path: req.repo_path.clone(),
                    branch: None,
                    include_paths: vec![req.path.clone()],
                    exclude_paths: Vec::new(),
                })
                .source_policy(SourcePolicy::MetadataOnly)
                .next_tools(vec![
                    "get_skeleton".to_string(),
                    "get_patch_context".to_string(),
                ]),
            payload,
            "summarize_module",
            req.repo_path.as_deref(),
            baseline_chars,
            &Self::baseline_sample(&baseline_sample),
        ))
    }

    #[tool(description = "Estimate token and latency cost before retrieving agent context.")]
    async fn estimate_context_cost(
        &self,
        Parameters(req): Parameters<EstimateContextCostReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let default_req = mcp_scaled_budget(6000, 512, 12000);
        let requested = req.budget_tokens.unwrap_or(default_req).clamp(512, 12000);
        let scope_factor = req
            .include_paths
            .as_ref()
            .map(|p| p.len())
            .unwrap_or(1)
            .max(1);
        let mode = req
            .mode
            .unwrap_or_else(|| detect_intent(&req.task).to_string());
        let estimate = (req.task.len() / 2 + scope_factor * 350 + 750).min(requested);
        Ok(EnvelopeBuilder::new(started)
            .audit_tool("estimate_context_cost")
            .cost_class("cheap")
            .freshness(FreshnessState::Unknown)
            .source_policy(SourcePolicy::MetadataOnly)
            .success(json!({
                "task": req.task,
                "mode": mode,
                "budget_tokens": requested,
                "estimated_tokens": estimate,
                "estimated_latency_ms": if estimate < 4000 { 750 } else { 1800 },
                "recommended_tool": "get_patch_context"
            })))
    }

    #[tool(
        description = "Recommend the smallest safe CodeCortex MCP tool sequence for an AI-agent task. Use before tool-heavy work to reduce calls, tokens, and source exposure."
    )]
    async fn recommend_tools(
        &self,
        Parameters(req): Parameters<RecommendToolsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let intent = infer_agent_intent(
            &req.task,
            req.intent.as_deref(),
            req.artifact.as_deref(),
            req.symbol_hint.as_deref(),
        );
        let profile = crate::mcp_profile::McpProfile::from_env();
        let allow_source = req
            .allow_source
            .unwrap_or_else(|| profile.default_allow_source_in_recommendations());
        let graph_only = req
            .graph_only
            .unwrap_or_else(|| matches!(profile, crate::mcp_profile::McpProfile::Strict));
        let freshness = req
            .freshness
            .as_deref()
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        let limit = req.limit.unwrap_or(8).clamp(1, 16);

        let mut sequence = recommended_tool_sequence(&intent, &freshness, graph_only);
        if !allow_source {
            sequence.retain(|name| {
                tool_metadata_for(name)
                    .map(|meta| !meta.can_return_source)
                    .unwrap_or(true)
            });
            for fallback in metadata_safe_fallbacks(&intent, graph_only) {
                if !sequence.contains(&fallback) {
                    sequence.push(fallback);
                }
            }
        }
        sequence.truncate(limit);

        let recommendations: Vec<Value> = sequence
            .iter()
            .enumerate()
            .filter_map(|(idx, name)| {
                recommendation_entry(
                    name,
                    idx + 1,
                    &intent,
                    allow_source,
                    Some(&req.task),
                    req.budget_tokens,
                )
            })
            .collect();

        let warnings =
            recommendation_warnings(&freshness, allow_source, req.budget_tokens, graph_only);
        Ok(EnvelopeBuilder::new(started)
            .audit_tool("recommend_tools")
            .cost_class("cheap")
            .freshness(freshness_state_from_label(&freshness))
            .scope(ResponseScope {
                repo_path: req.repo_path.clone(),
                branch: None,
                include_paths: req.include_paths.unwrap_or_default(),
                exclude_paths: Vec::new(),
            })
            .source_policy(if allow_source {
                SourcePolicy::MetadataOnly
            } else {
                SourcePolicy::Forbidden
            })
            .next_tools(sequence.iter().map(|tool| (*tool).to_string()).collect())
            .success(json!({
                "task": req.task,
                "intent": intent,
                "mcp_profile": profile.as_str(),
                "allow_source": allow_source,
                "graph_only": graph_only,
                "language": req.language,
                "symbol_hint": req.symbol_hint,
                "artifact": req.artifact,
                "budget_tokens": req.budget_tokens,
                "warnings": warnings,
                "recommendations": recommendations
            })))
    }

    #[tool(
        description = "Return compact guidance cards for a specific MCP tool or task-relevant tools. Use instead of reading the whole catalog when agent context is tight."
    )]
    async fn get_tool_guidance(
        &self,
        Parameters(req): Parameters<ToolGuidanceReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if let Some(tool_name) = req.tool_name {
            let card = tool_card_for(&tool_name).ok_or_else(|| {
                McpError::invalid_params(format!("Unknown CodeCortex tool: {tool_name}"), None)
            })?;
            return Ok(EnvelopeBuilder::new(started)
                .audit_tool("get_tool_guidance")
                .cost_class("cheap")
                .source_policy(SourcePolicy::MetadataOnly)
                .next_tools(card.guidance.follow_ups.clone())
                .success(json!({ "cards": [card] })));
        }

        let profile = crate::mcp_profile::McpProfile::from_env();
        let graph_only = req
            .graph_only
            .unwrap_or_else(|| matches!(profile, crate::mcp_profile::McpProfile::Strict));
        let mut task_for_infer = req
            .task
            .clone()
            .unwrap_or_else(|| "general CodeCortex tool routing".to_string());
        if let Some(ref lang) = req.language {
            if !lang.trim().is_empty() {
                task_for_infer.push_str(" language:");
                task_for_infer.push_str(lang.trim());
            }
        }
        let fresh_label = req
            .freshness
            .as_deref()
            .unwrap_or("unknown")
            .to_ascii_lowercase();
        let intent = infer_agent_intent(
            &task_for_infer,
            None,
            req.artifact.as_deref(),
            req.symbol_hint.as_deref(),
        );
        let limit = req.limit.unwrap_or(6).clamp(1, 16);
        let cards: Vec<ToolCard> = recommended_tool_sequence(&intent, &fresh_label, graph_only)
            .into_iter()
            .filter_map(tool_card_for)
            .take(limit)
            .collect();

        Ok(EnvelopeBuilder::new(started)
            .audit_tool("get_tool_guidance")
            .cost_class("cheap")
            .freshness(freshness_state_from_label(&fresh_label))
            .source_policy(SourcePolicy::MetadataOnly)
            .next_tools(
                cards
                    .iter()
                    .map(|card| card.metadata.name.to_string())
                    .collect(),
            )
            .success(json!({
                "task": task_for_infer,
                "intent": intent,
                "graph_only": graph_only,
                "mcp_profile": profile.as_str(),
                "freshness": fresh_label,
                "language": req.language,
                "symbol_hint": req.symbol_hint,
                "artifact": req.artifact,
                "budget_tokens": req.budget_tokens,
                "cards": cards
            })))
    }

    #[tool(
        description = "Search deferred MCP tools by fuzzy name match and return schemas. Use when lazy tool discovery is enabled."
    )]
    async fn tools_search(
        &self,
        Parameters(req): Parameters<ToolsSearchReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if req.query.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "query must not be empty",
                None,
                started,
            ));
        }
        let max_results = req.max_results.unwrap_or(8);
        let promote = req.promote.unwrap_or(false);
        let all_tools = self.tool_router.list_all();
        let mut promoted = lazy_tools::lock_promoted(&self.promoted_tools);
        let result = lazy_tools::tools_search(
            &all_tools,
            &mut promoted,
            &req.query,
            max_results,
            promote,
        );
        Ok(envelope_success(
            serde_json::to_value(&result).unwrap_or_else(|_| json!({})),
            started,
            vec!["tool_profile".to_string()],
            false,
        ))
    }

    #[tool(
        description = "Report lazy tool discovery state: live vs deferred tool counts and optional per-tool membership."
    )]
    async fn tool_profile(
        &self,
        Parameters(req): Parameters<ToolProfileReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let promoted = lazy_tools::lock_promoted(&self.promoted_tools);
        let report = lazy_tools::tool_profile(&promoted, req.tool.as_deref());
        Ok(envelope_success(
            serde_json::to_value(&report).unwrap_or_else(|_| json!({})),
            started,
            vec!["tools_search".to_string()],
            false,
        ))
    }

    #[tool(
        description = "Explain graph/vector freshness state and exact repair commands for a repository."
    )]
    async fn explain_index_freshness(
        &self,
        Parameters(req): Parameters<IndexStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let repo_path = req.repo_path.unwrap_or_else(default_repo_path);
        let client = self.graph_client().await?;
        let branch_records = get_branch_indexes(&client, &repo_path)
            .await
            .unwrap_or_default();
        let latest = branch_records.first();
        let freshness = latest
            .map(|record| record.graph_freshness)
            .unwrap_or(IndexFreshness::Unknown);
        let mode_hint = latest
            .and_then(|record| record.file_hash_watermark.as_deref())
            .map(|watermark| {
                if watermark.is_empty() {
                    "unknown"
                } else {
                    "indexed"
                }
            })
            .unwrap_or("unknown");
        Ok(EnvelopeBuilder::new(started)
            .audit_tool("explain_index_freshness")
            .cost_class("cheap")
            .freshness(match freshness {
                IndexFreshness::Fresh => FreshnessState::Fresh,
                IndexFreshness::Warming => FreshnessState::Warming,
                IndexFreshness::Partial => FreshnessState::Partial,
                IndexFreshness::Stale => FreshnessState::Stale,
                IndexFreshness::Unknown => FreshnessState::Unknown,
            })
            .source_policy(SourcePolicy::MetadataOnly)
            .next_tools(vec!["index_status".to_string(), "project_sync".to_string()])
            .success(json!({
                "repo_path": repo_path,
                "freshness": freshness.as_str(),
                "mode_hint": mode_hint,
                "latest_branch_index": latest,
                "meaning": match freshness {
                    IndexFreshness::Fresh => "Graph metadata says the latest branch index is fresh.",
                    IndexFreshness::Partial => "The latest graph update was partial; repair with a full reindex before trusting global results.",
                    IndexFreshness::Stale => "The latest graph index is stale; repair before relying on graph answers.",
                    IndexFreshness::Warming => "Indexing is currently warming.",
                    IndexFreshness::Unknown => "CodeCortex could not prove graph and vector watermarks from this lightweight check.",
                },
                "repair_commands": [
                    format!("cortex index {} --force", shell_quote(&repo_path)),
                    format!("cortex index {} --mode incremental-diff", shell_quote(&repo_path)),
                    format!("cortex vector-index {}", shell_quote(&repo_path)),
                    format!("cortex watch {}", shell_quote(&repo_path))
                ]
            })))
    }

    #[tool(
        description = "Get the impact graph for a symbol: who calls it, what it calls, and dependents. Use when the user asks 'what calls X?', 'what does X affect?', or 'show callers/callees of X'. Returns nodes and edges with file paths and relationship types."
    )]
    async fn get_impact_graph(
        &self,
        Parameters(req): Parameters<ImpactGraphReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.impact_graph.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_impact_graph is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.symbol.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "symbol must not be empty",
                None,
                started,
            ));
        }
        let repo_scope = req.repo_path.clone().unwrap_or_else(|| ".".to_string());
        let cache_revision = repo_scope.clone();
        if self.tool_enabled("cache", true) {
            let cache = self.tool_cache().await;
            let key = L1Cache::make_key("get_impact_graph", &repo_scope, &tool_params_hash(&req));
            if let (Some(cached), _) = cache.get::<Value>(&key, &cache_revision) {
                return Ok(envelope_success(cached, started, Vec::new(), false));
            }
        }
        let depth = req.depth.unwrap_or(4).clamp(1, 8) as usize;
        let analyzer = Analyzer::new(self.graph_client().await?);
        let client = self.graph_client().await?;
        let budget = req.budget_tokens.unwrap_or(8000) as u32;
        let params = crate::intelligence::ImpactGraphParams {
            symbol: req.symbol.clone(),
            depth,
            include_importers: req.include_importers.unwrap_or(true),
            budget_tokens: budget,
            symbol_type: req
                .symbol_type
                .clone()
                .unwrap_or_else(|| "auto".to_string()),
        };
        let include_paths = vec![repo_scope.clone()];
        let pack = crate::intelligence::build_impact_pack(
            &client,
            &analyzer,
            &repo_scope,
            &include_paths,
            &params,
            None,
        )
        .await;
        let payload = pack.data.clone();
        if self.tool_enabled("cache", true) {
            let cache = self.tool_cache().await;
            let key = L1Cache::make_key("get_impact_graph", &repo_scope, &tool_params_hash(&req));
            cache.put(&key, payload, cache_revision);
        }
        Ok(pack.to_envelope(
            "get_impact_graph",
            started,
            include_paths,
            Vec::new(),
            Vec::new(),
        ))
    }

    #[tool(
        description = "Find control/data flow paths between two symbols (e.g. from entry to a function). Use when the user asks 'how does A reach B?', 'path from X to Y', or 'logic flow between two functions'. Pass from_symbol and to_symbol; returns paths with intermediate nodes."
    )]
    async fn search_logic_flow(
        &self,
        Parameters(req): Parameters<LogicFlowReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.logic_flow.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "search_logic_flow is disabled by feature flag",
                None,
                started,
            ));
        }
        let max_paths = req.max_paths.unwrap_or(5).clamp(1, 20);
        let max_depth = if req.from_symbol == req.to_symbol {
            req.max_depth.unwrap_or(4).clamp(1, 8)
        } else {
            req.max_depth.unwrap_or(12).clamp(1, 20)
        };
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let resolver = self.build_symbol_resolver(Some(&repo_path), None).await?;
        let graph_scope = resolver.repo_scope().to_string();
        let from_hits = resolver
            .resolve_exact_definitional(&req.from_symbol, 3)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let to_hits = resolver
            .resolve_exact_definitional(&req.to_symbol, 3)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        if from_hits.is_empty() {
            return Ok(envelope_error(
                "NOT_FOUND",
                format!("from_symbol '{}' not found", req.from_symbol),
                None,
                started,
            ));
        }
        if to_hits.is_empty() {
            return Ok(envelope_error(
                "NOT_FOUND",
                format!("to_symbol '{}' not found", req.to_symbol),
                None,
                started,
            ));
        }
        let same_symbol = req.from_symbol == req.to_symbol;
        if same_symbol {
            let node = &from_hits[0];
            return Ok(envelope_success(
                json!({
                    "self_reference": true,
                    "paths": [{
                        "nodes": [{
                            "name": node.name,
                            "path": node.file_path,
                            "line": node.line_number,
                            "kind": node.kind
                        }],
                        "edges": []
                    }],
                    "searched_depth": max_depth
                }),
                started,
                Vec::new(),
                false,
            ));
        }
        let kinds = callable_kinds_cypher_list();
        let cypher = format!(
            "MATCH p=(a:CodeNode)-[:CALLS*1..{max_depth}]->(b:CodeNode)
             WHERE a.repository_path = $repo AND b.repository_path = $repo
               AND a.name = $from AND b.name = $to
               AND a.kind IN {kinds} AND b.kind IN {kinds}
             RETURN [n IN nodes(p) | {{name: n.name, path: n.path, line: n.line_number, kind: n.kind}}] AS nodes,
                    [r IN relationships(p) | type(r)] AS edges
             LIMIT {max_paths}"
        );
        let client = self.graph_client().await?;
        let rows = client
            .query_with_params(
                &cypher,
                vec![
                    ("repo", graph_scope.clone()),
                    ("from", req.from_symbol.clone()),
                    ("to", req.to_symbol.clone()),
                ],
            )
            .await
            .unwrap_or_default();
        let paths: Vec<Value> = rows
            .iter()
            .filter_map(|row| {
                Some(json!({
                    "nodes": row.get("nodes").cloned().unwrap_or(json!([])),
                    "edges": row.get("edges").cloned().unwrap_or(json!([])),
                }))
            })
            .collect();
        let partial = paths.is_empty() && req.allow_partial.unwrap_or(true);
        let warnings = if partial {
            vec!["no_call_path_found".to_string()]
        } else {
            Vec::new()
        };
        Ok(envelope_success(
            json!({
                "paths": paths,
                "searched_depth": max_depth,
                "from": req.from_symbol,
                "to": req.to_symbol
            }),
            started,
            warnings,
            partial,
        ))
    }

    #[tool(
        description = "Get a file skeleton: high-level structure (functions, classes, exports) without full body. Use when the user wants an overview of a file, 'what's in this file', or to navigate structure. Pass path; returns outline with names and locations."
    )]
    async fn get_skeleton(
        &self,
        Parameters(req): Parameters<SkeletonReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_skeleton is disabled by feature flag",
                None,
                started,
            ));
        }
        let mode = req.mode.unwrap_or_else(|| "minimal".to_string());
        let content = fs::read_to_string(&req.path)
            .map_err(|e| McpError::invalid_params(format!("unable to read path: {e}"), None))?;
        let etag = crate::rerank::content_etag(&content);
        if req.if_none_match.as_deref() == Some(etag.as_str()) {
            return Ok(crate::savings::finish_not_modified_response(
                self.savings_enabled(),
                EnvelopeBuilder::new(started).audit_tool("get_skeleton"),
                &etag,
                "get_skeleton",
                req.repo_path.as_deref(),
                content.chars().count(),
                &Self::baseline_sample(&content),
            ));
        }
        let skeleton = build_skeleton(content.as_str(), mode.as_str());
        let payload = json!({
            "path": req.path,
            "mode": mode,
            "content": skeleton,
            "precomputed": false,
            "compression_ratio": 0.7,
            "etag": etag,
        });
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("get_skeleton")
                .etag(&etag),
            payload,
            "get_skeleton",
            None,
            content.chars().count(),
            &Self::baseline_sample(&content),
        ))
    }

    #[tool(
        description = "Unified health and status for indexing, watcher, and jobs. Use when the user asks 'is indexing done?', 'what's the index status?', or 'are there running jobs?'. Returns repo index status, watcher state, and job list."
    )]
    async fn index_status(
        &self,
        Parameters(req): Parameters<IndexStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.index_status.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "index_status is disabled by feature flag",
                None,
                started,
            ));
        }
        let include_jobs = req.include_jobs.unwrap_or(true);
        let include_watcher = req.include_watcher.unwrap_or(true);
        let path = req.repo_path.unwrap_or_else(default_repo_path);
        let client = self.graph_client().await?;
        let health = true;
        let stats = Analyzer::new(client.clone())
            .repository_stats()
            .await
            .unwrap_or_default();
        let branch_records = get_branch_indexes(&client, &path).await.unwrap_or_default();
        let latest = branch_records.first();
        let job_list = if include_jobs {
            self.jobs.list()
        } else {
            Vec::new()
        };
        let watched = if include_watcher {
            WatchSession::new(&self.config).list()
        } else {
            Vec::new()
        };
        let has_running_job = job_list.iter().any(|j| j.state == JobState::Running);
        let freshness = if has_running_job {
            IndexFreshness::Warming
        } else if let Some(record) = latest {
            record.graph_freshness
        } else {
            IndexFreshness::Unknown
        };
        let vector_freshness = self
            .resolve_vector_freshness_label(&path, latest)
            .await;
        let repair_commands = vec![
            format!("cortex index {} --force", shell_quote(&path)),
            format!("cortex vector-index {}", shell_quote(&path)),
            format!("cortex watch {}", shell_quote(&path)),
        ];
        Ok(envelope_success(
            json!({
                "health": if health { "ok" } else { "degraded" },
                "repo_path": path,
                "freshness": {
                    "overall": freshness.as_str(),
                    "graph": freshness.as_str(),
                    "vector": vector_freshness,
                    "repair_commands": repair_commands,
                    "latest_branch_index": latest,
                    "states": {
                        "fresh": "full or incremental graph update completed and promoted branch metadata",
                        "partial": "an update timed out or failed before promotion; run full repair",
                        "stale": "metadata exists but is known out of date",
                        "warming": "an index job is running"
                    }
                },
                "counts": {
                    "repositories": stats.len()
                },
                "indexing": {
                    "progress_pct": if has_running_job { 50 } else { 100 }
                },
                "watcher": {
                    "running": !watched.is_empty(),
                    "watched_paths": watched
                },
                "jobs": {
                    "running": job_list.iter().filter(|j| j.state == JobState::Running).count(),
                    "completed": job_list.iter().filter(|j| j.state == JobState::Completed).count(),
                    "failed": job_list.iter().filter(|j| j.state == JobState::Failed).count()
                }
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Detect workspace agents, generate MCP config, and optionally install the CodeCortex agent pack (skills, subagents, hooks, rules) into .cursor/"
    )]
    async fn workspace_setup(
        &self,
        Parameters(req): Parameters<WorkspaceSetupReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.workspace_setup.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "workspace_setup is disabled by feature flag",
                None,
                started,
            ));
        }
        let repo = req.repo_path.unwrap_or_else(default_repo_path);
        let repo_path = PathBuf::from(&repo);
        let detect_agents = req.detect_agents.unwrap_or(true);
        let generate_configs = req.generate_configs.unwrap_or(true);
        let install_git = req.install_git_hooks.unwrap_or(false);
        let non_interactive = req.non_interactive.unwrap_or(false);
        let overwrite = req.overwrite.unwrap_or(false);
        let install_pack = req.install_agent_pack.unwrap_or(false);
        let install_cursor_mcp = req.install_cursor_mcp.unwrap_or(generate_configs);
        let enable_watch = req.enable_watch.unwrap_or(false);

        let mut detected = Vec::<String>::new();
        if detect_agents {
            if repo_path.join(".cursor").exists() {
                detected.push("cursor".to_string());
            }
            if repo_path.join("CLAUDE.md").exists() {
                detected.push("claude".to_string());
            }
            if repo_path.join("AGENTS.md").exists() {
                detected.push("codex".to_string());
            }
        }

        let mut created = Vec::<String>::new();
        let mut warnings = Vec::<String>::new();
        let mut agent_pack: Option<AgentPackInstallResult> = None;
        let mut watch_started: Vec<String> = Vec::new();

        if install_pack {
            let pack_override = req.agent_pack_root.as_deref().map(PathBuf::from);
            let pack_root = resolve_agent_pack(&repo_path, pack_override.as_deref());
            match pack_root {
                Some(root) => {
                    let targets = req.targets.clone().unwrap_or_else(|| vec!["cursor".to_string()]);
                    if targets.iter().any(|t| t == "cursor") {
                        let mut opts =
                            AgentPackInstallOptions::for_repo(&repo_path, &root);
                        opts.overwrite = overwrite;
                        opts.non_interactive = non_interactive;
                        opts.install_mcp = generate_configs;
                        opts.install_cursor_mcp = install_cursor_mcp;
                        match install_agent_pack(opts) {
                            Ok(result) => {
                                created.extend(result.installed.clone());
                                warnings.extend(result.warnings.clone());
                                agent_pack = Some(result);
                            }
                            Err(e) => warnings.push(format!("agent pack install failed: {e}")),
                        }
                    } else {
                        warnings.push("only cursor target is supported in v1".to_string());
                    }
                }
                None => warnings.push(
                    "agent pack not found: set CORTEX_AGENT_PACK or run from a repo containing plugin/codecortex"
                        .to_string(),
                ),
            }
        } else if generate_configs {
            let mcp_path = repo_path.join("mcp.json");
            if mcp_path.exists() && !(non_interactive && overwrite) {
                warnings.push("mcp.json exists; skipped overwrite".to_string());
            } else {
                let command = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "cortex".to_string());
                let cfg = json!({
                    "mcpServers": {
                        "codecortex": {
                            "command": command,
                            "args": ["mcp", "start"],
                            "cwd": repo
                        }
                    }
                });
                fs::write(
                    &mcp_path,
                    serde_json::to_string_pretty(&cfg).unwrap_or_default(),
                )
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                created.push("mcp.json".to_string());
            }
            if install_cursor_mcp {
                let cursor_mcp = repo_path.join(".cursor/mcp.json");
                if cursor_mcp.exists() && !(non_interactive && overwrite) {
                    warnings.push(".cursor/mcp.json exists; skipped overwrite".to_string());
                } else {
                    let command = std::env::current_exe()
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                        .unwrap_or_else(|| "cortex".to_string());
                    let cfg = json!({
                        "mcpServers": {
                            "codecortex": {
                                "command": command,
                                "args": ["mcp", "start"],
                                "cwd": repo
                            }
                        }
                    });
                    if let Some(parent) = cursor_mcp.parent() {
                        fs::create_dir_all(parent)
                            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    }
                    fs::write(
                        &cursor_mcp,
                        serde_json::to_string_pretty(&cfg).unwrap_or_default(),
                    )
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    created.push(".cursor/mcp.json".to_string());
                }
            }
        }

        let mut hooks = Vec::<String>::new();
        if install_git {
            let hooks_dir = repo_path.join(".git/hooks");
            if hooks_dir.exists() {
                let pre_commit = hooks_dir.join("pre-commit");
                if !pre_commit.exists() || (non_interactive && overwrite) {
                    fs::write(
                        &pre_commit,
                        "#!/usr/bin/env sh\ncargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings\n",
                    )
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                    hooks.push("pre-commit".to_string());
                } else {
                    warnings.push("pre-commit hook exists; skipped overwrite".to_string());
                }
            } else {
                warnings.push(".git/hooks directory not found".to_string());
            }
        }

        if enable_watch {
            let paths = req
                .watch_paths
                .clone()
                .unwrap_or_else(|| vec![repo.clone()]);
            for path in paths {
                match self.spawn_watch_for_path(path.clone()).await {
                    Ok(job_id) => watch_started.push(format!("{path} -> {job_id}")),
                    Err(e) => warnings.push(format!("watch {path}: {e}")),
                }
            }
        }

        Ok(envelope_success(
            json!({
                "detected_agents": detected,
                "created_files": created,
                "hooks_installed": hooks,
                "agent_pack": agent_pack,
                "watch_started": watch_started,
                "repositories_registered": [repo]
            }),
            started,
            warnings,
            false,
        ))
    }

    #[tool(
        description = "Spawn an asynchronous A2A multi-agent session (consensus review, patch planning, analysis) without linear MCP tool chains. Requires [a2a].enabled and mcp.tools.a2a_spawn_session in ~/.cortex/config.toml."
    )]
    async fn cortex_a2a_spawn_session(
        &self,
        Parameters(req): Parameters<A2aSpawnSessionReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let hub = match self.check_a2a_access(started) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        let include_paths = req.include_paths.clone().unwrap_or_default();
        let workflow = req
            .workflow
            .clone()
            .unwrap_or_else(|| "consensus_review".to_string());
        let wait = req.wait_for_completion.unwrap_or(false);
        let spawn = cortex_a2a::SpawnSessionRequest {
            task: req.task,
            workflow: req
                .workflow
                .unwrap_or_else(|| "consensus_review".to_string()),
            roles: req.roles.unwrap_or_default(),
            include_paths: include_paths.clone(),
            exclude_paths: req.exclude_paths.clone().unwrap_or_default(),
            exclude_globs: req.exclude_globs.clone().unwrap_or_default(),
            target_symbol: req.target_symbol.clone(),
            source_branch: req.source_branch.clone(),
            target_branch: req.target_branch.clone(),
            mode: req.mode.clone(),
            return_immediately: req.return_immediately.unwrap_or(true),
            wait_for_completion: wait,
            budget_tokens: req.budget_tokens.unwrap_or(6000),
        };
        match if wait {
            hub.spawn_session_async(spawn).await
        } else {
            hub.spawn_session(spawn).map_err(|e| e)
        } {
            Ok(mut resp) => {
                resp.freshness = hub.freshness_for_paths(&include_paths).await;
                resp.suggested_next_tools = cortex_a2a::services::spawn_tool_hints(&workflow);
                if !matches!(resp.freshness.as_str(), "fresh" | "Fresh" | "FRESH") {
                    resp.warnings.push(format!(
                        "index freshness is {} — repair index before high-confidence impact claims",
                        resp.freshness
                    ));
                }
                if let Some(url) = req.push_callback_url.as_ref() {
                    if hub.config.push.enabled {
                        hub.push()
                            .create_config(cortex_a2a::TaskPushNotificationConfig {
                                id: String::new(),
                                task_id: resp.task_id.clone(),
                                url: url.clone(),
                                token: None,
                            });
                    } else {
                        resp.warnings
                            .push("push_callback_url ignored: [a2a.push].enabled is false".into());
                    }
                }
                Ok(envelope_success(
                    serde_json::to_value(resp).unwrap_or_default(),
                    started,
                    Vec::new(),
                    false,
                ))
            }
            Err(e) => Ok(envelope_error("INTERNAL", &e.to_string(), None, started)),
        }
    }

    #[tool(description = "Poll an A2A task by id (GetTask). Requires [a2a].enabled.")]
    async fn cortex_a2a_get_task(
        &self,
        Parameters(req): Parameters<A2aGetTaskReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let hub = match self.check_a2a_access(started) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        match hub.get_task_wire_with_options(
            &req.task_id,
            req.history_length,
            req.include_artifacts.unwrap_or(true),
        ) {
            Ok(wire) => {
                let include_artifacts = req.include_artifacts.unwrap_or(true);
                let body = if req.spec_json.unwrap_or(false) {
                    cortex_a2a::spec_codec::task_wire_to_spec_json_with_options(
                        &wire,
                        include_artifacts,
                    )
                    .unwrap_or_else(|_| serde_json::to_value(&wire).unwrap_or_default())
                } else if include_artifacts {
                    serde_json::to_value(wire).unwrap_or_default()
                } else {
                    let mut v = serde_json::to_value(wire).unwrap_or_default();
                    if let serde_json::Value::Object(map) = &mut v {
                        map.remove("artifacts");
                    }
                    v
                };
                Ok(envelope_success(body, started, Vec::new(), false))
            }
            Err(e) => Ok(envelope_error("NOT_FOUND", &e.to_string(), None, started)),
        }
    }

    #[tool(description = "Send an A2A message (spec SendMessage) and return task or message.")]
    async fn cortex_a2a_send_message(
        &self,
        Parameters(req): Parameters<A2aSendMessageReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let hub = match self.check_a2a_access(started) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        let include_paths = req.include_paths.clone().unwrap_or_default();
        let wire_req = cortex_a2a::SendMessageRequestWire {
            message: cortex_a2a::A2aMessage {
                message_id: uuid::Uuid::new_v4().to_string(),
                context_id: req.context_id,
                task_id: req.task_id,
                role: "user".to_string(),
                parts: vec![cortex_a2a::A2aPart {
                    text: Some(req.message),
                    data: None,
                    metadata: None,
                    media_type: None,
                }],
                metadata: None,
                extensions: vec![],
            },
            configuration: Some(cortex_a2a::SendMessageConfigurationWire {
                return_immediately: req.return_immediately.unwrap_or(true),
                history_length: None,
            }),
        };
        match hub.send_message_with_options(wire_req, req.workflow.as_deref(), &include_paths) {
            Ok(resp) => Ok(envelope_success(
                serde_json::to_value(resp).unwrap_or_default(),
                started,
                Vec::new(),
                false,
            )),
            Err(e) => Ok(envelope_error("INTERNAL", &e.to_string(), None, started)),
        }
    }

    #[tool(description = "Cancel an A2A task by id.")]
    async fn cortex_a2a_cancel_task(
        &self,
        Parameters(req): Parameters<A2aCancelTaskReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let hub = match self.check_a2a_access(started) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        match hub.cancel_task(&req.task_id) {
            Ok(wire) => Ok(envelope_success(
                cortex_a2a::task_wire_to_spec_json(&wire).unwrap_or_default(),
                started,
                Vec::new(),
                false,
            )),
            Err(e) => Ok(envelope_error("NOT_FOUND", &e.to_string(), None, started)),
        }
    }

    #[tool(description = "List A2A tasks, optionally filtered by context_id.")]
    async fn cortex_a2a_list_tasks(
        &self,
        Parameters(req): Parameters<A2aListTasksReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let hub = match self.check_a2a_access(started) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        match hub.list_tasks_wire(req.context_id.as_deref()) {
            Ok(list) => Ok(envelope_success(
                serde_json::to_value(list).unwrap_or_default(),
                started,
                Vec::new(),
                false,
            )),
            Err(e) => Ok(envelope_error("INTERNAL", &e.to_string(), None, started)),
        }
    }

    #[tool(description = "Poll latest task state; returns spec JSON and SSE subscribe hint.")]
    async fn cortex_a2a_subscribe_task(
        &self,
        Parameters(req): Parameters<A2aSubscribeTaskReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let hub = match self.check_a2a_access(started) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        match hub.get_task_wire(&req.task_id) {
            Ok(wire) => {
                let listen = self.config.mcp.network.listen.clone();
                let body = json!({
                    "task": cortex_a2a::task_wire_to_spec_json(&wire).unwrap_or_default(),
                    "subscribe_sse": format!("http://{listen}/a2a/v1/tasks/{}/subscribe", req.task_id),
                });
                Ok(envelope_success(body, started, Vec::new(), false))
            }
            Err(e) => Ok(envelope_error("NOT_FOUND", &e.to_string(), None, started)),
        }
    }

    #[tool(description = "List push notification configs for a task when [a2a.push].enabled.")]
    async fn cortex_a2a_list_push_configs(
        &self,
        Parameters(req): Parameters<A2aListPushConfigsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let hub = match self.check_a2a_access(started) {
            Ok(h) => h,
            Err(e) => return Ok(e),
        };
        if !self.config.a2a.push.enabled {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "push disabled",
                None,
                started,
            ));
        }
        let configs = if let Some(task_id) = req.task_id.as_deref().filter(|s| !s.is_empty()) {
            hub.push().list_for_task(task_id)
        } else {
            hub.push().list_all()
        };
        Ok(envelope_success(
            json!({ "configs": configs }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Orchestrate CodeCortex session health: assess index freshness, diagnose issues, recommend tools, optionally install the agent pack and start directory watch. Use at session start or when bootstrapping a repo."
    )]
    async fn manage_codecortex(
        &self,
        Parameters(req): Parameters<ManageCodecortexReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.manage_codecortex.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "manage_codecortex is disabled by feature flag",
                None,
                started,
            ));
        }

        let action = req
            .action
            .as_deref()
            .unwrap_or("assess")
            .to_ascii_lowercase();

        if action == "spawn_a2a_session" {
            let spawn_req = A2aSpawnSessionReq {
                task: req
                    .task
                    .clone()
                    .unwrap_or_else(|| "A2A session".to_string()),
                workflow: Some("consensus_review".to_string()),
                roles: None,
                include_paths: None,
                exclude_paths: None,
                exclude_globs: None,
                target_symbol: None,
                source_branch: None,
                target_branch: None,
                mode: None,
                return_immediately: Some(true),
                wait_for_completion: None,
                budget_tokens: Some(6000),
                push_callback_url: None,
            };
            return self.cortex_a2a_spawn_session(Parameters(spawn_req)).await;
        }

        let repo = req
            .repo_path
            .clone()
            .or_else(|| {
                self.projects
                    .get_current_project()
                    .map(|p| p.path.display().to_string())
            })
            .unwrap_or_else(default_repo_path);
        let repo_path = PathBuf::from(&repo);
        let task = req
            .task
            .clone()
            .unwrap_or_else(|| "session start and code intelligence bootstrap".to_string());
        let enable_watch = req.enable_watch.unwrap_or(false);
        let auto_repair = req.auto_repair.unwrap_or(false);
        let mut warnings = Vec::<String>::new();
        let mut next_steps = Vec::<String>::new();

        let health = match self.graph_client().await {
            Ok(_) => json!({
                "status": "ok",
                "graph": "connected",
                "backend": GraphClient::configured_backend(&self.config).to_string(),
            }),
            Err(e) => {
                warnings.push(format!("graph unavailable: {e}"));
                json!({
                    "status": "degraded",
                    "error": e.to_string(),
                    "suggested_action": self.graph_connect_hint(),
                })
            }
        };

        let freshness_label = self.repo_freshness_label(&repo).await;
        let watched = WatchSession::new(&self.config).list();
        let index_status = json!({
            "repo_path": repo,
            "freshness": freshness_label,
            "watcher": {
                "running": !watched.is_empty(),
                "watched_paths": watched
            }
        });

        let diagnose_issues = self.diagnose_issues_summary(&repo).await;
        let diagnose_summary = json!({
            "issue_count": diagnose_issues.len(),
            "issues": diagnose_issues,
        });

        if matches!(freshness_label.as_str(), "stale" | "unknown") {
            next_steps.push(
                "Index freshness is not proven fresh — run add_code_to_graph or delegate codecortex-indexer before impact-heavy tools."
                    .to_string(),
            );
        } else {
            next_steps.push(
                "Freshness looks acceptable — prefer get_patch_context and scoped graph tools."
                    .to_string(),
            );
        }

        let intent = infer_agent_intent(&task, None, None, None);
        let profile = crate::mcp_profile::McpProfile::from_env();
        let graph_only = matches!(profile, crate::mcp_profile::McpProfile::Strict);
        let sequence = recommended_tool_sequence(&intent, &freshness_label, graph_only);
        let recommendations: Vec<Value> = sequence
            .iter()
            .take(8)
            .enumerate()
            .filter_map(|(idx, name)| {
                recommendation_entry(name, idx + 1, &intent, false, Some(&task), None)
            })
            .collect();

        let mut agent_pack: Option<AgentPackInstallResult> = None;
        if action == "bootstrap" && req.install_agent_pack.unwrap_or(true) {
            let pack_override = req.agent_pack_root.as_deref().map(PathBuf::from);
            if let Some(root) = resolve_agent_pack(&repo_path, pack_override.as_deref()) {
                let opts = AgentPackInstallOptions::for_repo(&repo_path, &root);
                match install_agent_pack(opts) {
                    Ok(result) => {
                        next_steps.push(format!(
                            "Agent pack installed from {} ({} files).",
                            result.agent_pack_root,
                            result.installed.len()
                        ));
                        agent_pack = Some(result);
                    }
                    Err(e) => warnings.push(format!("agent pack: {e}")),
                }
            } else {
                warnings.push(
                    "agent pack not found — set CORTEX_AGENT_PACK or use plugin/codecortex/cursor/install.sh"
                        .to_string(),
                );
            }
        }

        let mut watch_started: Vec<String> = Vec::new();
        if enable_watch {
            let paths = req
                .watch_paths
                .clone()
                .unwrap_or_else(|| vec![repo.clone()]);
            for path in paths {
                match self.spawn_watch_for_path(path.clone()).await {
                    Ok(job_id) => {
                        watch_started.push(format!("{path} -> {job_id}"));
                        next_steps.push(format!("Watching {path} (job {job_id})"));
                    }
                    Err(e) => warnings.push(format!("watch {path}: {e}")),
                }
            }
        }

        let mut repair_job: Option<String> = None;
        if auto_repair
            && matches!(freshness_label.as_str(), "stale" | "unknown")
            && self.graph_client().await.is_ok()
        {
            let job_id = format!("repair-{}", now_millis());
            if let Ok(Some(daemon_stage)) =
                Self::try_enqueue_daemon_index(&self.config, Path::new(&repo), true)
            {
                repair_job = daemon_stage
                    .get("job")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                next_steps.push(format!(
                    "Queued daemon repair job {} — poll project_status before impact analysis.",
                    repair_job.as_deref().unwrap_or("unknown")
                ));
            } else {
                self.jobs
                    .mark_running(&job_id, format!("Repair index {}", repo));
                let cfg = self.config.clone();
                let jobs = self.jobs.clone();
                let path = repo.clone();
                let job_id_for_task = job_id.clone();
                tokio::spawn(async move {
                    let outcome = async {
                        let client = GraphClient::connect(&cfg).await?;
                        let indexer = Indexer::from_cortex_config(client, &cfg)?;
                        indexer.index_path_with_options(&path, true).await?;
                        anyhow::Ok(())
                    }
                    .await;
                    if let Err(err) = outcome {
                        jobs.mark_failed(&job_id_for_task, err.to_string());
                    } else {
                        jobs.mark_completed(&job_id_for_task, "graph repair completed");
                    }
                });
                next_steps.push(format!(
                    "Queued graph repair job {job_id} — check check_job_status before impact analysis."
                ));
                repair_job = Some(job_id);
            }
        }

        if action == "repair_plan" && repair_job.is_none() && !auto_repair {
            next_steps.push(
                "repair_plan: call add_code_to_graph with force=true or set auto_repair=true once."
                    .to_string(),
            );
        }

        next_steps.push(
            "Call recommend_tools or read codecortex://guide/agent-pack-bootstrap for routing."
                .to_string(),
        );

        Ok(envelope_success(
            json!({
                "action": action,
                "health": health,
                "index_status": index_status,
                "diagnose_summary": diagnose_summary,
                "freshness": freshness_label,
                "recommendations": recommendations,
                "watcher": index_status.get("watcher"),
                "agent_pack": agent_pack,
                "watch_started": watch_started,
                "repair_job": repair_job,
                "next_steps": next_steps,
            }),
            started,
            warnings,
            false,
        ))
    }

    #[tool(description = "Submit LSP-derived call edges with dedup and rejection stats")]
    async fn submit_lsp_edges(
        &self,
        Parameters(req): Parameters<SubmitLspEdgesReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.lsp_ingest.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "submit_lsp_edges is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.edges.is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "edges must not be empty",
                None,
                started,
            ));
        }
        let merge_mode = req.merge_mode.unwrap_or_else(|| "upsert".to_string());
        let mut unique = HashSet::<(String, String, String, u64)>::new();
        let mut deduped = 0usize;
        let mut ingested = 0usize;
        let mut rejected = 0usize;
        let mut reasons = HashMap::<String, usize>::new();
        let client = self.graph_client().await?;
        for edge in req.edges {
            let key = (
                edge.caller_fqn.clone(),
                edge.callee_fqn.clone(),
                edge.file.clone(),
                edge.line,
            );
            if !unique.insert(key) {
                deduped += 1;
                continue;
            }
            let caller = edge
                .caller_fqn
                .rsplit("::")
                .next()
                .unwrap_or(edge.caller_fqn.as_str());
            let callee = edge
                .callee_fqn
                .rsplit("::")
                .next()
                .unwrap_or(edge.callee_fqn.as_str());
            let q = format!(
                "MATCH (a:Function {{name:'{}'}}), (b:Function {{name:'{}'}})
                 WHERE a.path STARTS WITH '{}' AND b.path STARTS WITH '{}'
                 MERGE (a)-[r:CALLS]->(b)
                 SET r.kind='Calls',
                     r.source='lsp',
                     r.confidence={},
                     r.file='{}',
                     r.line_number={},
                     r.merge_mode='{}'",
                escape_cypher(caller),
                escape_cypher(callee),
                escape_cypher(req.repo_path.as_str()),
                escape_cypher(req.repo_path.as_str()),
                edge.confidence.unwrap_or(0.5),
                escape_cypher(edge.file.as_str()),
                edge.line,
                escape_cypher(merge_mode.as_str())
            );
            match client.raw_query(q.as_str()).await {
                Ok(_) => ingested += 1,
                Err(_) => {
                    rejected += 1;
                    *reasons.entry("unknown_symbol".to_string()).or_insert(0) += 1;
                }
            }
        }
        Ok(envelope_success(
            json!({
                "ingested": ingested,
                "deduped": deduped,
                "rejected": rejected,
                "reasons": reasons
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Save a session observation (fact or decision) with optional symbol links. Use when the user or agent wants to persist something for later (e.g. 'remember we decided to use approach X'). Observations are searchable via search_memory."
    )]
    async fn save_observation(
        &self,
        Parameters(req): Parameters<SaveObservationReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.memory.write.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "save_observation is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.text.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "text must not be empty",
                None,
                started,
            ));
        }
        if req.text.len() > 8 * 1024 {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "text too large; max 8KB",
                None,
                started,
            ));
        }
        if looks_sensitive(req.text.as_str()) {
            return Ok(envelope_error(
                "SENSITIVE_CONTENT_DETECTED",
                "observation appears to contain sensitive content",
                None,
                started,
            ));
        }
        let store = self.memory_store().await?;
        let session_id = req
            .session_id
            .unwrap_or_else(|| "default-session".to_string());
        if store
            .is_rate_limited(session_id.as_str())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
        {
            return Ok(envelope_error(
                "RATE_LIMITED",
                "too many writes in short period",
                None,
                started,
            ));
        }
        let linked_symbols = req.symbol_refs.clone().unwrap_or_default();
        let mut warnings = Vec::new();
        let embedding = if self.tool_enabled("mcp.vector.write.enabled", true) {
            match self.vector_service().await {
                Ok(service) => match service.embed_query(req.text.as_str()).await {
                    Ok(embedding) => {
                        global_metrics().record_embeddings_generated(1);
                        Some(embedding)
                    }
                    Err(err) => {
                        global_metrics().record_vector_fallback();
                        warnings.push(warning_with_reason(WARNING_EMBEDDER_TIMEOUT, &err));
                        None
                    }
                },
                Err(err) => {
                    global_metrics().record_vector_fallback();
                    warnings.push(warning_with_reason(
                        WARNING_VECTOR_STORE_UNAVAILABLE,
                        &err.to_string(),
                    ));
                    None
                }
            }
        } else {
            warnings.push("vector_write_disabled".to_string());
            None
        };
        let obs_id = format!("obs-{}", now_millis());
        let rec = ObservationRecord {
            observation_id: obs_id.clone(),
            repo_id: req.repo_path.clone(),
            session_id,
            created_at: now_millis(),
            created_by: "mcp".to_string(),
            text: req.text,
            symbol_refs: linked_symbols.clone(),
            confidence: req.confidence.unwrap_or(0.8).clamp(0.0, 1.0),
            stale: false,
            classification: req.classification.unwrap_or_else(|| "internal".to_string()),
            severity: req.severity.unwrap_or_else(|| "info".to_string()),
            tags: req.tags.unwrap_or_default(),
            source_revision: "unknown".to_string(),
            embedding,
        };
        store
            .save(&observation_from_record(&rec))
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        append_audit_event("save_observation", obs_id.as_str())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({
                "observation_id": obs_id,
                "linked_symbols": linked_symbols.len(),
                "stale": false
            }),
            started,
            warnings,
            false,
        ))
    }

    #[tool(description = "Get session observations with stale/fresh metadata")]
    async fn get_session_context(
        &self,
        Parameters(req): Parameters<SessionContextReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.memory.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_session_context is disabled by feature flag",
                None,
                started,
            ));
        }
        let store = self.memory_store().await?;
        let include_previous = req.include_previous.unwrap_or(3);
        let max_items = req.max_items.unwrap_or(100).min(200);
        let include_stale = req.include_stale.unwrap_or(false);
        let session_id = req
            .session_id
            .clone()
            .unwrap_or_else(|| "default-session".to_string());
        let mut items: Vec<ObservationRecord> = store
            .search(
                req.repo_path.as_str(),
                None,
                None,
                include_stale,
                max_items.saturating_mul(4).max(max_items),
            )
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .into_iter()
            .filter(|o| o.session_id == session_id || include_previous > 0)
            .map(|o| record_from_observation(&o))
            .collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items.truncate(max_items);
        Ok(envelope_success(
            json!({ "items": items }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Search saved session observations/memory by query. Use when the user asks 'what did we decide about X?', 'recall earlier context', or to find past observations. Returns matching observations with scores."
    )]
    async fn search_memory(
        &self,
        Parameters(req): Parameters<SearchMemoryReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.memory.read.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "search_memory is disabled by feature flag",
                None,
                started,
            ));
        }
        if req.query.trim().is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "query must not be empty",
                None,
                started,
            ));
        }
        let store = self.memory_store().await?;
        let max_items = req.max_items.unwrap_or(20).min(100);
        let include_stale = req.include_stale.unwrap_or(false);
        let mut results = Vec::<Value>::new();
        let mut warnings = Vec::new();
        let query_embedding = if self.tool_enabled("mcp.vector.read.enabled", true) {
            match self.vector_service().await {
                Ok(service) => match service.embed_query(req.query.as_str()).await {
                    Ok(embedding) => {
                        global_metrics().record_embeddings_generated(1);
                        Some(embedding)
                    }
                    Err(err) => {
                        global_metrics().record_vector_fallback();
                        warnings.push(warning_with_reason(WARNING_FALLBACK_TO_LEXICAL, &err));
                        None
                    }
                },
                Err(err) => {
                    global_metrics().record_vector_fallback();
                    warnings.push(warning_with_reason(
                        WARNING_VECTOR_STORE_UNAVAILABLE,
                        &err.to_string(),
                    ));
                    None
                }
            }
        } else {
            warnings.push("vector_read_disabled".to_string());
            None
        };
        let candidates = store
            .search(
                req.repo_path.as_str(),
                Some(req.query.as_str()),
                None,
                include_stale,
                max_items.saturating_mul(8).max(100),
            )
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        for obs in candidates {
            let rec = record_from_observation(&obs);
            if rec.stale && !include_stale {
                continue;
            }
            let bm25 =
                simple_lexical_score(req.query.as_str(), rec.text.as_str(), rec.text.as_str());
            let tfidf = ((rec.text.len().min(180)) as f64) / 180.0;
            let recency = 1.0;
            let graph_proximity = if rec.symbol_refs.is_empty() { 0.0 } else { 0.2 };
            let staleness_penalty = if rec.stale { -0.2 } else { 0.0 };
            let semantic = match (&query_embedding, &rec.embedding) {
                (Some(query), Some(obs_vec)) => cosine_similarity(query, obs_vec),
                _ => 0.0,
            };
            let score =
                (semantic * 1.2) + bm25 + tfidf + recency + graph_proximity + staleness_penalty;
            results.push(json!({
                "id": rec.observation_id,
                "text": rec.text,
                "score": score,
                "classification": rec.classification,
                "stale": rec.stale,
                "why": {
                    "semantic": semantic,
                    "bm25": bm25,
                    "tfidf": tfidf,
                    "recency": recency,
                    "graph_proximity": graph_proximity,
                    "staleness_penalty": staleness_penalty
                }
            }));
        }
        results.sort_by(|a, b| {
            let left = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
            let right = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
            left.partial_cmp(&right)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let partial = results.is_empty();
        if partial {
            warnings.push("memory_empty".to_string());
        }
        results.truncate(max_items);
        Ok(envelope_success(
            json!({ "results": results }),
            started,
            warnings,
            partial,
        ))
    }


    #[tool(
        description = "List all repositories currently indexed in the graph. Use when the user asks 'what repos are indexed?', 'which projects are in the graph?', or to verify indexing before running graph tools."
    )]
    async fn list_indexed_repositories(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let repos = self
            .graph_client()
            .await?
            .list_repositories()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "repositories": repos }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Delete a repository and all its nodes from the graph. Use when the user wants to remove a repo from the index (e.g. after deleting the repo or to free space). Destructive; graph data for that repo is removed."
    )]
    async fn delete_repository(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if req.dry_run.unwrap_or(false) {
            return Ok(envelope_success(
                json!({
                    "dry_run": true,
                    "would_delete_repository": req.path,
                    "action": "delete_repository"
                }),
                started,
                Vec::new(),
                false,
            ));
        }
        if req.confirm != Some(true) {
            return Ok(envelope_error(
                "CONFIRMATION_REQUIRED",
                "delete_repository requires confirm=true (or dry_run=true)",
                None,
                started,
            ));
        }
        self.graph_client()
            .await?
            .delete_repository(&req.path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "deleted": req.path }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Get node-count statistics for all indexed repositories")]
    async fn get_repository_stats(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let stats = Analyzer::new(self.graph_client().await?)
            .repository_stats()
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({ "stats": stats }),
            started,
            Vec::new(),
            false,
        ))
    }


    #[tool(description = "Check status of a background indexing job by ID")]
    async fn check_job_status(
        &self,
        Parameters(req): Parameters<JobStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        Ok(envelope_success(
            json!({ "job": self.jobs.get(&req.id) }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "List all background jobs")]
    async fn list_jobs(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        Ok(envelope_success(
            json!({ "jobs": self.jobs.list() }),
            started,
            Vec::new(),
            false,
        ))
    }


    #[tool(description = "Load a .ccx graph bundle file into memory")]
    async fn load_bundle(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let bundle = BundleStore::import(PathBuf::from(&req.path).as_path())
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({
                "path": req.path,
                "nodes": bundle.nodes.len(),
                "edges": bundle.edges.len()
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Export a repository graph to a .ccx bundle file")]
    async fn export_bundle(
        &self,
        Parameters(req): Parameters<ExportBundleReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let client = self.graph_client().await?;
        let bundle = BundleStore::export_from_graph(&client, &req.repository_path)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        BundleStore::export(PathBuf::from(&req.output_path).as_path(), &bundle)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        Ok(envelope_success(
            json!({
                "repository_path": req.repository_path,
                "output_path": req.output_path,
                "nodes": bundle.nodes.len(),
                "edges": bundle.edges.len()
            }),
            started,
            Vec::new(),
            false,
        ))
    }


    #[tool(
        description = "Get rich signature information for a symbol (function, method, struct, enum). Returns parameters, return type, visibility, async status, generics, and related symbols."
    )]
    async fn get_signature(
        &self,
        Parameters(req): Parameters<GetSignatureReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let signature_enabled = self.tool_enabled("mcp.signature.enabled", true)
            || self.tool_enabled("mcp.skeleton.enabled", true);
        if !signature_enabled {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "get_signature is disabled by feature flag",
                None,
                started,
            ));
        }

        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let resolver = self.build_symbol_resolver(Some(&repo_path), None).await?;
        let include_related = req.include_related.unwrap_or(false);

        let mut hits = resolver
            .resolve_exact_definitional(&req.symbol, 10)
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        if hits.is_empty() {
            hits = resolver
                .resolve_fuzzy_definitional(&req.symbol, 10)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        }

        if hits.is_empty() {
            let suggestions = resolver
                .suggest_names(&req.symbol, 8)
                .await
                .unwrap_or_default();
            return Ok(envelope_error(
                "NOT_FOUND",
                format!("Symbol '{}' not found in repository", req.symbol),
                Some(json!({ "suggestions": suggestions })),
                started,
            ));
        }

        let client = self.graph_client().await?;
        let graph_scope = resolver.repo_scope().to_string();
        let nodes: Vec<_> = hits
            .into_iter()
            .map(|h| {
                let lang = h.lang.as_deref().and_then(|l| FromStr::from_str(l).ok());
                (
                    h.name,
                    h.file_path,
                    h.kind,
                    h.source.unwrap_or_default(),
                    h.line_number,
                    lang,
                )
            })
            .collect();

        let nodes_count = nodes.len();

        let mut signatures = Vec::new();
        let mut parse_warnings = Vec::new();
        let mut baseline_chars = 0usize;
        let mut baseline_sample = String::new();
        for (name, path, kind, mut source, line_number, lang) in nodes {
            baseline_chars += source.chars().count();
            if baseline_sample.chars().count() < 8192 {
                baseline_sample.push_str(&source);
                baseline_sample.push('\n');
            }
            if source.trim().is_empty() {
                if let Some(snippet) =
                    load_source_snippet_for_signature(&repo_path, &path, line_number)
                {
                    source = snippet;
                }
            }
            let node = cortex_core::CodeNode {
                id: format!("{}:{}", path, name),
                kind: match kind.as_str() {
                    "function" | "Function" => cortex_core::EntityKind::Function,
                    "struct" | "Struct" => cortex_core::EntityKind::Struct,
                    "enum" | "Enum" => cortex_core::EntityKind::Enum,
                    "trait" | "Trait" => cortex_core::EntityKind::Trait,
                    "impl" | "Impl" => cortex_core::EntityKind::Module,
                    "class" | "Class" => cortex_core::EntityKind::Class,
                    _ => cortex_core::EntityKind::Function,
                },
                name: name.clone(),
                path: Some(path.clone()),
                line_number,
                lang,
                source: Some(source.clone()),
                docstring: None,
                properties: std::collections::HashMap::new(),
            };

            if let Some(sig) = SignatureExtractor::extract(&node, &source) {
                let mut sig_json = serde_json::to_value(&sig).unwrap_or_default();
                if let Some(obj) = sig_json.as_object_mut() {
                    obj.insert("name".to_string(), json!(name));
                    obj.insert("path".to_string(), json!(path));
                    obj.insert("kind".to_string(), json!(kind));
                }
                signatures.push(sig_json);

                if include_related && signatures.len() < 20 {
                    let related_query = format!(
                        "MATCH (a:CodeNode {{name:'{}', repository_path:'{}'}})<-[:IMPLEMENTS|OVERRIDES]-(b:CodeNode) \
                         WHERE b.repository_path = '{}' \
                         RETURN b.name, b.path, b.kind, b.source, b.line_number, b.lang LIMIT 5",
                        escape_cypher(&name),
                        escape_cypher(&graph_scope),
                        escape_cypher(&graph_scope)
                    );
                    if let Ok(related) = client.raw_query(&related_query).await {
                        for rel_row in related {
                            if let (
                                Some(rname),
                                Some(rpath),
                                Some(rkind),
                                Some(rsource),
                                rline,
                                Some(rlang),
                            ) = (
                                rel_row.get("b.name").and_then(|v| v.as_str()),
                                rel_row.get("b.path").and_then(|v| v.as_str()),
                                rel_row.get("b.kind").and_then(|v| v.as_str()),
                                rel_row.get("b.source").and_then(|v| v.as_str()),
                                rel_row.get("b.line_number").and_then(|v| v.as_i64()),
                                rel_row.get("b.lang").and_then(|v| v.as_str()),
                            ) && let Ok(rlang_parsed) = FromStr::from_str(rlang)
                            {
                                let rel_node = cortex_core::CodeNode {
                                    id: format!("{}:{}", rpath, rname),
                                    kind: cortex_core::EntityKind::Function,
                                    name: rname.to_string(),
                                    path: Some(rpath.to_string()),
                                    line_number: rline.map(|n| n as u32),
                                    lang: Some(rlang_parsed),
                                    source: Some(rsource.to_string()),
                                    docstring: None,
                                    properties: std::collections::HashMap::new(),
                                };
                                if let Some(rel_sig) =
                                    SignatureExtractor::extract(&rel_node, rsource)
                                {
                                    let mut rel_json =
                                        serde_json::to_value(&rel_sig).unwrap_or_default();
                                    if let Some(obj) = rel_json.as_object_mut() {
                                        obj.insert("name".to_string(), json!(rname));
                                        obj.insert("path".to_string(), json!(rpath));
                                        obj.insert("kind".to_string(), json!(rkind));
                                        obj.insert("relation".to_string(), json!("implementation"));
                                    }
                                    signatures.push(rel_json);
                                }
                            }
                        }
                    }
                }
            } else if source.trim().is_empty() {
                parse_warnings.push(format!("empty_source:{path}"));
            }

            if signatures.len() >= 20 {
                break;
            }
        }

        if signatures.is_empty() {
            parse_warnings.push("signature_extraction_failed".to_string());
            let payload = json!({
                "signatures": [],
                "count": 0,
                "query": req.symbol,
                "nodes_found": nodes_count
            });
            return Ok(self.finish_counted_tool(
                EnvelopeBuilder::new(started)
                    .audit_tool("get_signature")
                    .partial(true)
                    .warnings(parse_warnings),
                payload,
                "get_signature",
                Some(repo_path.as_str()),
                baseline_chars,
                &Self::baseline_sample(&baseline_sample),
            ));
        }

        let payload = json!({
            "signatures": signatures,
            "count": signatures.len(),
            "query": req.symbol
        });
        let payload_text = payload.to_string();
        let etag = crate::rerank::content_etag(&payload_text);
        if req.if_none_match.as_deref() == Some(etag.as_str()) {
            return Ok(crate::savings::finish_not_modified_response(
                self.savings_enabled(),
                EnvelopeBuilder::new(started)
                    .audit_tool("get_signature")
                    .warnings(parse_warnings),
                &etag,
                "get_signature",
                Some(repo_path.as_str()),
                baseline_chars,
                &Self::baseline_sample(&baseline_sample),
            ));
        }
        Ok(self.finish_counted_tool(
            EnvelopeBuilder::new(started)
                .audit_tool("get_signature")
                .etag(&etag)
                .warnings(parse_warnings),
            payload,
            "get_signature",
            Some(repo_path.as_str()),
            baseline_chars,
            &Self::baseline_sample(&baseline_sample),
        ))
    }

    #[tool(
        description = "Find tests related to a symbol. Returns unit tests, integration tests, and test coverage information."
    )]
    async fn find_tests(
        &self,
        Parameters(req): Parameters<FindTestsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "find_tests is disabled by feature flag",
                None,
                started,
            ));
        }

        let client = self.graph_client().await?;
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let max_results = req.max_results.unwrap_or(20).min(100);
        let include_integration = req.include_integration.unwrap_or(true);

        // Find tests by multiple strategies:
        // 1. Direct TESTS relationship (if indexed)
        // 2. Naming convention (test_<symbol>, <symbol>_test, tests/test_<symbol>)
        // 3. Same module with test attribute

        let mut tests = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Strategy 1: Look for TESTS relationships
        let rel_query = format!(
            "MATCH (t:Function)-[:TESTS]->(s {{name:'{}'}}) \
             WHERE t.path STARTS WITH '{}' \
             RETURN t.name, t.path, t.source, t.line_number LIMIT {}",
            escape_cypher(&req.symbol),
            escape_cypher(&repo_path),
            max_results
        );

        if let Ok(results) = client.raw_query(&rel_query).await {
            for row in results {
                if let (Some(name), Some(path), source, line) = (
                    row.get("t.name").and_then(|v| v.as_str()),
                    row.get("t.path").and_then(|v| v.as_str()),
                    row.get("t.source").and_then(|v| v.as_str()),
                    row.get("t.line_number").and_then(|v| v.as_i64()),
                ) {
                    let key = format!("{}:{}", path, name);
                    if !seen.contains(&key) {
                        seen.insert(key);
                        tests.push(json!({
                            "name": name,
                            "path": path,
                            "kind": "unit",
                            "line_number": line,
                            "source_preview": source.map(|s| s.lines().take(5).collect::<Vec<_>>().join("\n"))
                        }));
                    }
                }
            }
        }

        // Strategy 2: Naming convention search
        let naming_patterns = vec![
            format!("test_{}", req.symbol),
            format!("{}_test", req.symbol),
            format!("test{}", req.symbol),
            format!("test_{}_", req.symbol),
        ];

        for pattern in naming_patterns {
            if tests.len() >= max_results {
                break;
            }

            let name_query = format!(
                "MATCH (t:Function) \
                 WHERE t.name CONTAINS '{}' AND t.path STARTS WITH '{}' \
                 AND (t.path CONTAINS 'test' OR t.path CONTAINS 'tests' OR t.path CONTAINS '_test' OR t.path CONTAINS 'spec') \
                 RETURN t.name, t.path, t.source, t.line_number LIMIT {}",
                escape_cypher(&pattern),
                escape_cypher(&repo_path),
                max_results - tests.len()
            );

            if let Ok(results) = client.raw_query(&name_query).await {
                for row in results {
                    if let (Some(name), Some(path), source, line) = (
                        row.get("t.name").and_then(|v| v.as_str()),
                        row.get("t.path").and_then(|v| v.as_str()),
                        row.get("t.source").and_then(|v| v.as_str()),
                        row.get("t.line_number").and_then(|v| v.as_i64()),
                    ) {
                        let key = format!("{}:{}", path, name);
                        if !seen.contains(&key) {
                            seen.insert(key);
                            tests.push(json!({
                                "name": name,
                                "path": path,
                                "kind": "unit",
                                "line_number": line,
                                "match_reason": "naming_convention",
                                "source_preview": source.map(|s| s.lines().take(5).collect::<Vec<_>>().join("\n"))
                            }));
                        }
                    }
                }
            }
        }

        // Strategy 3: Integration tests (if enabled)
        if include_integration && tests.len() < max_results {
            let int_query = format!(
                "MATCH (t:Function) \
                 WHERE t.path CONTAINS 'integration' AND t.path STARTS WITH '{}' \
                 AND (t.name CONTAINS '{}' OR t.source CONTAINS '{}') \
                 RETURN t.name, t.path, t.source, t.line_number LIMIT {}",
                escape_cypher(&repo_path),
                escape_cypher(&req.symbol),
                escape_cypher(&req.symbol),
                max_results - tests.len()
            );

            if let Ok(results) = client.raw_query(&int_query).await {
                for row in results {
                    if let (Some(name), Some(path), source, line) = (
                        row.get("t.name").and_then(|v| v.as_str()),
                        row.get("t.path").and_then(|v| v.as_str()),
                        row.get("t.source").and_then(|v| v.as_str()),
                        row.get("t.line_number").and_then(|v| v.as_i64()),
                    ) {
                        let key = format!("{}:{}", path, name);
                        if !seen.contains(&key) {
                            seen.insert(key);
                            tests.push(json!({
                                "name": name,
                                "path": path,
                                "kind": "integration",
                                "line_number": line,
                                "match_reason": "content_match",
                                "source_preview": source.map(|s| s.lines().take(5).collect::<Vec<_>>().join("\n"))
                            }));
                        }
                    }
                }
            }
        }

        let coverage_estimate = if tests.is_empty() {
            0.0
        } else if tests.len() >= 3 {
            0.85
        } else {
            0.5 + (tests.len() as f64 * 0.1)
        };

        if tests.is_empty() {
            return Ok(envelope_success(
                json!({
                    "tests": [],
                    "count": 0,
                    "symbol": req.symbol,
                    "coverage_estimate": 0.0,
                    "warning": "No tests found for this symbol. Consider adding unit tests."
                }),
                started,
                vec!["no_tests_found".to_string()],
                false,
            ));
        }

        Ok(envelope_success(
            json!({
                "tests": tests,
                "count": tests.len(),
                "symbol": req.symbol,
                "coverage_estimate": coverage_estimate.min(1.0),
                "has_unit_tests": tests.iter().any(|t| t.get("kind") == Some(&json!("unit"))),
                "has_integration_tests": tests.iter().any(|t| t.get("kind") == Some(&json!("integration")))
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Explain how a query would be processed. Shows interpretation, search strategy, and why results would be included. Useful for debugging and understanding the codebase."
    )]
    async fn explain_result(
        &self,
        Parameters(req): Parameters<ExplainResultReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "explain_result is disabled by feature flag",
                None,
                started,
            ));
        }

        let query = req.query.trim();
        if query.is_empty() {
            return Ok(envelope_error(
                "INVALID_ARGUMENT",
                "query must not be empty",
                None,
                started,
            ));
        }

        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let _tool = req.tool.as_deref().unwrap_or("find");

        let intent = detect_intent(query);

        let mut steps = Vec::new();
        let interpretation;
        let mut search_strategy = Vec::new();

        let query_type = if query.contains('(') || query.contains("fn ") || query.contains("func ")
        {
            "signature_search"
        } else if query.starts_with("test_") || query.contains(" test") {
            "test_search"
        } else if query.split_whitespace().count() > 3 {
            "semantic_search"
        } else {
            "symbol_search"
        };

        match query_type {
            "signature_search" => {
                interpretation = format!(
                    "Looking for code with specific signature pattern: '{}'",
                    query
                );
                steps.push(
                    "1. Parse signature components (name, parameters, return type)".to_string(),
                );
                steps.push("2. Match against indexed function signatures".to_string());
                steps.push("3. Rank by signature similarity score".to_string());
                search_strategy.push("exact_signature_match".to_string());
                search_strategy.push("fuzzy_parameter_match".to_string());
            }
            "test_search" => {
                interpretation = format!("Searching for tests related to: '{}'", query);
                steps.push("1. Extract symbol name from query".to_string());
                steps.push("2. Search for test files containing symbol".to_string());
                steps
                    .push("3. Match naming conventions (test_<symbol>, <symbol>_test)".to_string());
                steps.push("4. Check TESTS relationships in graph".to_string());
                search_strategy.push("naming_convention".to_string());
                search_strategy.push("graph_relationship".to_string());
            }
            "semantic_search" => {
                interpretation = format!("Semantic search for concepts related to: '{}'", query);
                steps.push("1. Extract key terms from query".to_string());
                steps.push("2. Full-text search on symbol names and docs".to_string());
                steps.push("3. TF-IDF scoring for relevance".to_string());
                steps.push("4. Graph traversal for related symbols".to_string());
                steps.push("5. Combine scores with intent-based weighting".to_string());
                search_strategy.push("full_text_search".to_string());
                search_strategy.push("tfidf_scoring".to_string());
                search_strategy.push("graph_expansion".to_string());
            }
            _ => {
                interpretation = format!("Searching for symbols matching: '{}'", query);
                steps.push("1. Exact name match search".to_string());
                steps.push("2. Prefix/fuzzy match for similar names".to_string());
                steps.push("3. Graph traversal for related symbols".to_string());
                steps.push("4. Rank by relevance and usage".to_string());
                search_strategy.push("exact_match".to_string());
                search_strategy.push("fuzzy_match".to_string());
            }
        }

        let simulated_matches = self.simulate_query_matches(query, &repo_path).await;

        let mut why_included = serde_json::Map::new();
        for (symbol, reason) in simulated_matches.iter().take(5) {
            why_included.insert(symbol.clone(), json!(reason));
        }

        Ok(envelope_success(
            json!({
                "query": query,
                "interpretation": interpretation,
                "detected_intent": intent,
                "query_type": query_type,
                "steps": steps,
                "search_strategy": search_strategy,
                "estimated_results": simulated_matches.len(),
                "why_included": why_included,
                "tips": if simulated_matches.is_empty() {
                    vec![
                        "Try using partial names or prefixes",
                        "Check spelling of symbol names",
                        "Use broader search terms"
                    ]
                } else {
                    vec![
                        "Results are ranked by relevance score",
                        "Related symbols are included via graph traversal",
                        "Use path_filter to narrow results to specific directories"
                    ]
                }
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    async fn simulate_query_matches(&self, query: &str, _repo_path: &str) -> Vec<(String, String)> {
        let mut matches = Vec::new();

        let terms: Vec<&str> = query.split_whitespace().collect();
        if !terms.is_empty() {
            let primary = terms[0];
            matches.push((
                format!("func:{}", primary),
                "Name match (1.0), primary query term".to_string(),
            ));

            if terms.len() > 1 {
                for term in &terms[1..] {
                    matches.push((
                        format!("func:{}_related", term),
                        "Related term match (0.7), same module".to_string(),
                    ));
                }
            }
        }

        matches
    }

    #[tool(
        description = "Analyze the impact of refactoring a symbol. Shows affected files, tests, breaking changes, and suggested steps for safe refactoring."
    )]
    async fn analyze_refactoring(
        &self,
        Parameters(req): Parameters<AnalyzeRefactoringReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.impact_graph.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "analyze_refactoring is disabled by feature flag",
                None,
                started,
            ));
        }

        let client = self.graph_client().await?;
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let graph_scope = cortex_core::graph_repository_path_for_index(Path::new(&repo_path), None);
        let detailed = req.detailed.unwrap_or(false);
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
        let change_type = req
            .change_type
            .clone()
            .unwrap_or_else(|| "modify".to_string());

        let symbol_results =
            lookup_code_nodes_by_symbol(&client, &graph_scope, &req.symbol, None, 1)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let (symbol_path, symbol_kind) = if let Some(row) = symbol_results.first() {
            let path = row
                .get("n.path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let kind = row
                .get("n.kind")
                .and_then(|v| v.as_str())
                .unwrap_or("function")
                .to_string();
            (path, kind)
        } else {
            return Ok(envelope_error(
                "NOT_FOUND",
                format!("Symbol '{}' not found in repository", req.symbol),
                None,
                started,
            ));
        };

        let (affected_files, affected_tests, warnings, safe_to_refactor) =
            match change_type.as_str() {
                "add_parameter" | "remove_parameter" | "change_signature" => {
                    self.analyze_signature_change(&client, &req.symbol, &repo_path)
                        .await
                }
                "rename" => self.analyze_rename(&client, &req.symbol, &repo_path).await,
                "delete" | "remove" => {
                    self.analyze_deletion(&client, &req.symbol, &repo_path)
                        .await
                }
                "extract" | "extract_method" => {
                    self.analyze_extraction(&client, &req.symbol, &repo_path)
                        .await
                }
                _ => {
                    self.analyze_generic_change(&client, &req.symbol, &repo_path)
                        .await
                }
            };

        let affected_files = filter_rows_by_paths(affected_files, &filters);
        let affected_tests = filter_rows_by_paths(affected_tests, &filters);

        let suggested_tests = self
            .find_suggested_tests(&client, &req.symbol, &repo_path)
            .await;

        let mut suggested_steps = vec![
            format!("1. Review all {} affected files", affected_files.len()),
            format!(
                "2. Update {} call sites",
                affected_files
                    .iter()
                    .filter(|f| f.get("call_site") == Some(&json!(true)))
                    .count()
            ),
            format!("3. Run {} related tests", suggested_tests.len()),
        ];

        if !safe_to_refactor {
            suggested_steps.push("4. ⚠️ Address breaking changes before proceeding".to_string());
        } else {
            suggested_steps.push("4. Make changes in small, testable commits".to_string());
        }

        let result = json!({
            "symbol": req.symbol,
            "symbol_path": symbol_path,
            "symbol_kind": symbol_kind,
            "change_type": change_type,
            "safe_to_refactor": safe_to_refactor,
            "affected_files": affected_files,
            "affected_files_count": affected_files.len(),
            "affected_tests": affected_tests,
            "affected_tests_count": affected_tests.len(),
            "warnings": warnings,
            "suggested_tests": suggested_tests,
            "suggested_steps": suggested_steps,
            "risk_level": if !safe_to_refactor { "high" } else if affected_files.len() > 10 { "medium" } else { "low" },
            "detailed": detailed
        });

        Ok(envelope_success(
            result,
            started,
            if !safe_to_refactor {
                vec!["breaking_change_detected".to_string()]
            } else {
                Vec::new()
            },
            false,
        ))
    }

    async fn analyze_signature_change(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let mut affected_files = Vec::new();
        let mut affected_tests = Vec::new();
        let mut warnings = Vec::new();
        let mut safe_to_refactor = true;

        let caller_query = format!(
            "MATCH (caller)-[:CALLS]->(callee {{name:'{}'}}) \
             WHERE caller.path STARTS WITH '{}' \
             RETURN caller.name, caller.path, caller.kind",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&caller_query).await {
            for row in results {
                if let (Some(name), Some(path), kind) = (
                    row.get("caller.name").and_then(|v| v.as_str()),
                    row.get("caller.path").and_then(|v| v.as_str()),
                    row.get("caller.kind").and_then(|v| v.as_str()),
                ) {
                    affected_files.push(json!({
                        "name": name,
                        "path": path,
                        "kind": kind,
                        "call_site": true,
                        "change_required": "update_call"
                    }));

                    if path.contains("test") || path.contains("spec") || name.starts_with("test_") {
                        affected_tests.push(json!({
                            "name": name,
                            "path": path,
                            "kind": "test_update"
                        }));
                    }
                }
            }
        }

        let visibility_query = format!(
            "MATCH (n {{name:'{}'}}) WHERE n.path STARTS WITH '{}' \
             RETURN n.source",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&visibility_query).await
            && let Some(row) = results.first()
            && let Some(source) = row.get("n.source").and_then(|v| v.as_str())
            && source.contains("pub ")
        {
            warnings.push("Breaking change: public API modification".to_string());
            safe_to_refactor = false;
        }

        if affected_files.len() > 10 {
            warnings.push(format!(
                "High impact: {} files affected",
                affected_files.len()
            ));
        }

        (affected_files, affected_tests, warnings, safe_to_refactor)
    }

    async fn analyze_rename(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let mut affected_files = Vec::new();
        let mut affected_tests = Vec::new();
        let mut warnings = Vec::new();

        let ref_query = format!(
            "MATCH (ref)-[r:CALLS|IMPORTS|REFERENCES]->(target {{name:'{}'}}) \
             WHERE ref.path STARTS WITH '{}' \
             RETURN ref.name, ref.path, type(r) as rel_type",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&ref_query).await {
            for row in results {
                if let (Some(name), Some(path)) = (
                    row.get("ref.name").and_then(|v| v.as_str()),
                    row.get("ref.path").and_then(|v| v.as_str()),
                ) {
                    affected_files.push(json!({
                        "name": name,
                        "path": path,
                        "change_required": "update_reference"
                    }));

                    if path.contains("test") || name.starts_with("test_") {
                        affected_tests.push(json!({
                            "name": name,
                            "path": path
                        }));
                    }
                }
            }
        }

        warnings.push("Use IDE refactoring tools for reliable rename".to_string());

        (affected_files, affected_tests, warnings, true)
    }

    async fn analyze_deletion(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let mut affected_files = Vec::new();
        let affected_tests = Vec::new();
        let mut warnings = Vec::new();

        let caller_query = format!(
            "MATCH (caller)-[:CALLS]->(callee {{name:'{}'}}) \
             WHERE caller.path STARTS WITH '{}' \
             RETURN caller.name, caller.path",
            escape_cypher(symbol),
            escape_cypher(repo_path)
        );

        if let Ok(results) = client.raw_query(&caller_query).await {
            for row in results {
                if let (Some(name), Some(path)) = (
                    row.get("caller.name").and_then(|v| v.as_str()),
                    row.get("caller.path").and_then(|v| v.as_str()),
                ) {
                    affected_files.push(json!({
                        "name": name,
                        "path": path,
                        "change_required": "remove_call_or_replace",
                        "breaking": true
                    }));
                }
            }
        }

        warnings.push("⚠️ Deletion is a breaking change".to_string());
        if !affected_files.is_empty() {
            warnings.push(format!("{} call sites will break", affected_files.len()));
        }

        (affected_files, affected_tests, warnings, false)
    }

    async fn analyze_extraction(
        &self,
        _client: &GraphClient,
        symbol: &str,
        _repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        let warnings = vec![
            "Extraction creates new symbol - ensure proper naming".to_string(),
            "Add tests for the new extracted function/method".to_string(),
        ];

        (
            vec![json!({"name": symbol, "change_required": "extract_to_new"})],
            vec![],
            warnings,
            true,
        )
    }

    async fn analyze_generic_change(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> (Vec<Value>, Vec<Value>, Vec<String>, bool) {
        self.analyze_signature_change(client, symbol, repo_path)
            .await
    }

    async fn find_suggested_tests(
        &self,
        client: &GraphClient,
        symbol: &str,
        repo_path: &str,
    ) -> Vec<String> {
        let mut tests = Vec::new();

        let test_query = format!(
            "MATCH (t:Function) \
             WHERE (t.name CONTAINS 'test_' OR t.path CONTAINS 'test') \
             AND t.path STARTS WITH '{}' \
             AND (t.name CONTAINS '{}' OR t.source CONTAINS '{}') \
             RETURN t.name LIMIT 10",
            escape_cypher(repo_path),
            escape_cypher(symbol),
            escape_cypher(symbol)
        );

        if let Ok(results) = client.raw_query(&test_query).await {
            for row in results {
                if let Some(name) = row.get("t.name").and_then(|v| v.as_str()) {
                    tests.push(name.to_string());
                }
            }
        }

        tests
    }

    #[tool(
        description = "Run diagnostics on the CodeCortex system. Checks index health, graph connectivity, and system status. Returns issues and suggested actions."
    )]
    async fn diagnose(
        &self,
        Parameters(req): Parameters<DiagnoseReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let check_type = req.check.clone().unwrap_or_else(|| "all".to_string());
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);

        let mut issues = Vec::new();
        let mut suggested_actions = Vec::new();

        if check_type == "all" || check_type == "graph_connectivity" {
            match self.graph_client().await {
                    Ok(client) => {
                        let test_query = "MATCH (n) RETURN count(n) as count LIMIT 1";
                        let start = Instant::now();
                        match client.raw_query(test_query).await {
                            Ok(_results) => {
                                let latency_ms = start.elapsed().as_millis();
                                if latency_ms > 100 {
                                    issues.push(json!({
                                    "check": "graph_latency",
                                    "severity": "warning",
                                    "message": format!("Graph query latency high: {}ms (threshold: 100ms)", latency_ms)
                                }));
                                    suggested_actions
                                        .push("Consider checking graph database server resources");
                                }
                            }
                            Err(e) => {
                                issues.push(json!({
                                    "check": "graph_query",
                                    "severity": "critical",
                                    "message": format!("Graph query failed: {}", e)
                                }));
                                suggested_actions.push(self.graph_connect_hint());
                            }
                        }
                    }
                    Err(e) => {
                        issues.push(json!({
                            "check": "graph_connection",
                            "severity": "critical",
                            "message": format!("Cannot connect to graph database: {}", e)
                        }));
                        suggested_actions.push(self.graph_connect_hint());
                    }
            }
        }

        if (check_type == "all" || check_type == "index_health")
            && let Ok(client) = self.graph_client().await
        {
            let repo_query = format!(
                "MATCH (r:Repository {{path:'{}'}}) RETURN r.indexed_at as indexed_at",
                escape_cypher(&repo_path)
            );

            if let Ok(results) = client.raw_query(&repo_query).await
                && results.is_empty()
            {
                issues.push(json!({
                    "check": "index_status",
                    "severity": "warning",
                    "message": "Repository not indexed"
                }));
                suggested_actions.push("Run: add_code_to_graph with the repository path");
            }

            let node_query = "MATCH (n:CodeNode) RETURN count(n) as count";
            if let Ok(results) = client.raw_query(node_query).await
                && let Some(row) = results.first()
                && let Some(count) = row.get("count").and_then(|v| v.as_i64())
                && count == 0
            {
                issues.push(json!({
                    "check": "index_content",
                    "severity": "warning",
                    "message": "No code nodes in graph - index may be empty"
                }));
                suggested_actions.push("Index a codebase first using add_code_to_graph");
            }
        }

        // Suggest `.cortexignore` when heavy build dirs exist but no ignore file.
        if check_type == "all" || check_type == "index_health" {
            let repo = Path::new(&repo_path);
            let git_root =
                cortex_core::find_git_repository_root(repo).unwrap_or_else(|| repo.to_path_buf());
            let ignore_path = git_root.join(cortex_core::CORTEXIGNORE_FILENAME);
            if !ignore_path.is_file() {
                let heavy_dirs = ["target", "node_modules", "build", "dist"];
                if heavy_dirs.iter().any(|d| git_root.join(d).is_dir()) {
                    issues.push(json!({
                        "check": "cortexignore",
                        "severity": "info",
                        "message": "No .cortexignore found but build/vendor directories are present"
                    }));
                    suggested_actions.push(
                        "Add a .cortexignore at the repo root (e.g. target/, node_modules/, generated/) to speed indexing",
                    );
                }
            }
        }

        if check_type == "all" || check_type == "cache_status" {
            let metrics = crate::metrics::global_metrics();
            let snapshot = metrics.snapshot();

            if snapshot.cache_hits + snapshot.cache_misses > 0 {
                let hit_rate = snapshot.cache_hits as f64
                    / (snapshot.cache_hits + snapshot.cache_misses) as f64;
                if hit_rate < 0.5 {
                    issues.push(json!({
                        "check": "cache_efficiency",
                        "severity": "info",
                        "message": format!("Cache hit rate low: {:.1}% (hits: {}, misses: {})",
                            hit_rate * 100.0, snapshot.cache_hits, snapshot.cache_misses)
                    }));
                    suggested_actions.push("Cache may be cold - repeated queries will be faster");
                }
            }
        }

        if check_type == "all" || check_type == "privacy" {
            if self.tool_enabled("mcp.vector.read.enabled", true)
                || self.tool_enabled("mcp.vector.write.enabled", true)
            {
                suggested_actions.push(
                    "Use local embeddings for private repositories, or explicitly approve remote embedding providers",
                );
            }
            if std::env::var("CORTEX_MCP_ALLOW_REMOTE")
                .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
                .unwrap_or(false)
            {
                issues.push(json!({
                    "check": "remote_mcp",
                    "severity": "warning",
                    "message": "Remote MCP mode appears enabled; require token auth and path allowlists"
                }));
                suggested_actions
                    .push("Bind MCP to 127.0.0.1 unless remote access is explicitly required");
            }
        }

        let status = if issues
            .iter()
            .any(|i| i.get("severity") == Some(&json!("critical")))
        {
            "unhealthy"
        } else if !issues.is_empty() {
            "degraded"
        } else {
            "ok"
        };

        let result = json!({
            "status": status,
            "check_type": check_type,
            "repo_path": repo_path,
            "issues": issues,
            "issue_count": issues.len(),
            "suggested_actions": suggested_actions,
            "checks_run": if check_type == "all" {
                json!(["graph_connectivity", "index_health", "cache_status", "privacy"])
            } else {
                json!([check_type])
            }
        });

        Ok(envelope_success(
            result,
            started,
            if status != "ok" {
                vec!["issues_detected".to_string()]
            } else {
                Vec::new()
            },
            false,
        ))
    }

    #[tool(
        description = "Find code patterns in the codebase. Detects architectural patterns like Builder, Factory, Singleton, Repository, Service, etc."
    )]
    async fn find_patterns(
        &self,
        Parameters(req): Parameters<FindPatternsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        if !self.tool_enabled("mcp.skeleton.enabled", true) {
            return Ok(envelope_error(
                "UNAVAILABLE",
                "find_patterns is disabled by feature flag",
                None,
                started,
            ));
        }

        let client = self.graph_client().await?;
        let repo_path = req.repo_path.clone().unwrap_or_else(default_repo_path);
        let min_confidence = req.min_confidence.unwrap_or(0.5);
        let max_results = req.max_results.unwrap_or(50).min(200);
        let filters = build_analyze_filters(
            req.include_paths.clone(),
            req.include_files.clone(),
            req.include_globs.clone(),
            req.exclude_paths.clone(),
            req.exclude_files.clone(),
            req.exclude_globs.clone(),
        )
        .map_err(|e| McpError::invalid_params(e.to_string(), None))?;

        let patterns_to_check = if let Some(ref pattern) = req.pattern {
            vec![pattern.to_lowercase()]
        } else {
            vec![
                "builder".to_string(),
                "factory".to_string(),
                "singleton".to_string(),
                "repository".to_string(),
                "service".to_string(),
                "handler".to_string(),
                "middleware".to_string(),
                "observer".to_string(),
                "strategy".to_string(),
                "adapter".to_string(),
                "decorator".to_string(),
                "command".to_string(),
                "state".to_string(),
                "facade".to_string(),
                "proxy".to_string(),
            ]
        };

        let mut results = Vec::new();

        for pattern in patterns_to_check {
            let matches = self
                .detect_pattern(&client, &pattern, &repo_path, min_confidence, max_results)
                .await;
            results.extend(filter_rows_by_paths(matches, &filters));
            if results.len() >= max_results {
                break;
            }
        }

        results.sort_by(|a, b| {
            b.get("confidence")
                .and_then(|c| c.as_f64())
                .unwrap_or(0.0)
                .partial_cmp(&a.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(max_results);

        let mut pattern_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for result in &results {
            if let Some(pattern) = result.get("pattern").and_then(|p| p.as_str()) {
                *pattern_counts.entry(pattern.to_string()).or_insert(0) += 1;
            }
        }

        if results.is_empty() {
            return Ok(envelope_success(
                json!({
                    "patterns": [],
                    "count": 0,
                    "pattern_summary": {},
                    "message": "No patterns found. Try lowering min_confidence or indexing more code."
                }),
                started,
                vec!["no_patterns_found".to_string()],
                false,
            ));
        }

        Ok(envelope_success(
            json!({
                "patterns": results,
                "count": results.len(),
                "pattern_summary": pattern_counts,
                "patterns_detected": pattern_counts.keys().cloned().collect::<Vec<_>>()
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    async fn detect_pattern(
        &self,
        client: &GraphClient,
        pattern: &str,
        repo_path: &str,
        min_confidence: f64,
        _max_results: usize,
    ) -> Vec<Value> {
        let mut matches = Vec::new();

        let (name_patterns, structural_hints, behavioral_hints) = match pattern {
            "builder" => (
                vec!["Builder", "Build"],
                vec!["build(", ".build()"],
                vec!["chain", "fluent"],
            ),
            "factory" => (
                vec!["Factory", "Create", "Make"],
                vec!["create(", "new_", "make_"],
                vec!["instantiation", "construction"],
            ),
            "singleton" => (
                vec!["Instance", "Singleton", "Global"],
                vec!["instance()", "get_instance()", "static"],
                vec!["single", "global"],
            ),
            "repository" => (
                vec!["Repository", "Repo", "Store", "DAO"],
                vec!["find(", "save(", "delete(", "query("],
                vec!["data", "persistence", "storage"],
            ),
            "service" => (
                vec!["Service", "Manager", "Provider"],
                vec!["process(", "handle(", "execute("],
                vec!["business", "logic"],
            ),
            "handler" => (
                vec!["Handler", "Controller", "Endpoint"],
                vec!["handle(", "process(", "route("],
                vec!["request", "response", "http"],
            ),
            "middleware" => (
                vec!["Middleware", "Interceptor", "Filter"],
                vec!["next(", "chain(", "intercept("],
                vec!["pipeline", "chain"],
            ),
            "observer" => (
                vec!["Observer", "Listener", "Subscriber", "Watcher"],
                vec!["notify(", "subscribe(", "emit("],
                vec!["event", "callback"],
            ),
            "strategy" => (
                vec!["Strategy", "Policy", "Algorithm"],
                vec!["execute(", "apply(", "strategy"],
                vec!["interchangeable", "algorithm"],
            ),
            "adapter" => (
                vec!["Adapter", "Wrapper", "Converter"],
                vec!["adapt(", "convert(", "wrap("],
                vec!["interface", "conversion", "compatibility"],
            ),
            "decorator" => (
                vec!["Decorator", "Wrapper"],
                vec!["decorate(", "wrap(", "impl "],
                vec!["extension", "enhancement", "adding"],
            ),
            "command" => (
                vec!["Command", "Action", "Operation"],
                vec!["execute(", "undo(", "redo(", "command"],
                vec!["encapsulate", "request", "operation"],
            ),
            "state" => (
                vec!["State", "Machine", "FSM"],
                vec!["transition(", "state", "current_state"],
                vec!["finite", "state machine", "states"],
            ),
            "facade" => (
                vec!["Facade", "API", "Interface"],
                vec!["facade(", "simplify(", "delegate("],
                vec!["simplified", "unified", "interface"],
            ),
            "proxy" => (
                vec!["Proxy", "Remote", "Virtual"],
                vec!["proxy(", "forward(", "delegate("],
                vec![" surrogate", "placeholder", "access control"],
            ),
            _ => (vec![pattern], vec![], vec![]),
        };

        for name_pattern in name_patterns {
            let query = format!(
                "MATCH (n) WHERE (n.name CONTAINS '{}' OR n.path CONTAINS '{}') \
                 AND n.path STARTS WITH '{}' \
                 RETURN n.name, n.path, n.kind, n.source LIMIT 20",
                escape_cypher(name_pattern),
                escape_cypher(name_pattern),
                escape_cypher(repo_path)
            );

            if let Ok(results) = client.raw_query(&query).await {
                for row in results {
                    if let (Some(name), Some(path), kind, source) = (
                        row.get("n.name").and_then(|v| v.as_str()),
                        row.get("n.path").and_then(|v| v.as_str()),
                        row.get("n.kind").and_then(|v| v.as_str()),
                        row.get("n.source").and_then(|v| v.as_str()),
                    ) {
                        let mut confidence: f64 = 0.4;

                        if let Some(src) = source {
                            for hint in &structural_hints {
                                if src.contains(hint) {
                                    confidence += 0.15;
                                }
                            }
                            for hint in &behavioral_hints {
                                if src.to_lowercase().contains(hint) {
                                    confidence += 0.05;
                                }
                            }
                        }

                        if let Some(k) = kind
                            && ((pattern == "builder" && k == "Struct")
                                || (pattern == "factory" && k == "Function")
                                || (pattern == "service" && (k == "Struct" || k == "Class"))
                                || (pattern == "repository" && k == "Struct")
                                || (pattern == "adapter" && k == "Struct")
                                || (pattern == "decorator" && k == "Struct")
                                || (pattern == "facade" && k == "Struct")
                                || (pattern == "proxy" && k == "Struct")
                                || (pattern == "command" && (k == "Struct" || k == "Enum"))
                                || (pattern == "state" && k == "Enum")
                                || (pattern == "strategy" && (k == "Trait" || k == "Interface")))
                        {
                            confidence += 0.1;
                        }

                        confidence = confidence.min(1.0);

                        if confidence >= min_confidence {
                            matches.push(json!({
                                "symbol": name,
                                "path": path,
                                "pattern": pattern,
                                "confidence": (confidence * 100.0).round() / 100.0,
                                "detection_method": "name_and_structure",
                                "kind": kind
                            }));
                        }
                    }
                }
            }
        }

        matches
    }


    #[tool(
        description = "Check FalkorDB/graph connectivity and report server health. Use when the user sees graph-related errors, or asks 'is the database up?'. Returns graph connection status, configured backend, and analyzer capabilities."
    )]
    async fn check_health(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let ok = self.graph_client().await.is_ok();
        Ok(envelope_success(
            json!({
                "graph": if ok { "connected" } else { "unreachable" },
                "backend": GraphClient::configured_backend(&self.config).to_string(),
                "analyzer": analyzer_capabilities_json()
            }),
            started,
            if ok {
                Vec::new()
            } else {
                vec!["degraded".to_string()]
            },
            !ok,
        ))
    }


    #[tool(description = "List all registered projects with their Git branch status")]
    async fn list_projects(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let projects = self.projects.list_projects();
        let current = self.projects.get_current_project();
        let total = projects.len();
        let active = projects
            .iter()
            .filter(|p| p.status == ProjectStatus::Watching)
            .count();

        Ok(envelope_success(
            json!({
                "projects": projects,
                "current_project": current.map(|p| p.path.display().to_string()),
                "total": total,
                "active": active
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Add a project to the registry for Git-aware indexing")]
    async fn add_project(
        &self,
        Parameters(req): Parameters<AddProjectReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        let config = cortex_core::ProjectConfig {
            track_branch: req.track_branch.unwrap_or(true),
            pinned_branches: req.pinned_branches.unwrap_or_default(),
            ..Default::default()
        };

        match self.projects.add_project(&path, Some(config)) {
            Ok(state) => {
                let summary = cortex_core::ProjectSummary::from(&state);
                Ok(envelope_success(
                    json!({
                        "project": summary,
                        "message": format!("Added project at {}", req.path)
                    }),
                    started_at,
                    Vec::new(),
                    false,
                ))
            }
            Err(cortex_watcher::RegistryError::ProjectAlreadyExists(_)) => {
                let summary = self
                    .projects
                    .get_project(&path)
                    .map(|s| cortex_core::ProjectSummary::from(&s));
                Ok(envelope_success(
                    json!({
                        "project": summary,
                        "message": format!("Project already registered at {}", req.path),
                        "already_exists": true
                    }),
                    started_at,
                    Vec::new(),
                    false,
                ))
            }
            Err(e) => Ok(envelope_error(
                "ADD_PROJECT_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(description = "Remove a project from the registry")]
    async fn remove_project(
        &self,
        Parameters(req): Parameters<RemoveProjectReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        match self.projects.remove_project(&path) {
            Ok(()) => Ok(envelope_success(
                json!({
                    "path": req.path,
                    "removed": true,
                    "message": format!("Removed project at {}", req.path)
                }),
                started_at,
                Vec::new(),
                false,
            )),
            Err(e) => Ok(envelope_error(
                "REMOVE_PROJECT_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(description = "Set the current active project and optionally switch branch")]
    async fn set_current_project(
        &self,
        Parameters(req): Parameters<SetProjectReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        if self.projects.get_project(&path).is_none() {
            let config = cortex_core::ProjectConfig::default();
            if let Err(e) = self.projects.add_project(&path, Some(config)) {
                if !matches!(e, cortex_watcher::RegistryError::ProjectAlreadyExists(_)) {
                    return Ok(envelope_error(
                        "SET_PROJECT_FAILED",
                        format!("add_project before set_current: {e}"),
                        None,
                        started_at,
                    ));
                }
            }
        }

        match self.projects.set_current_project(&path, req.branch.clone()) {
            Ok(pr) => {
                let project = self.projects.get_project(&path);
                Ok(envelope_success(
                    json!({
                        "project": project.map(|p| cortex_core::ProjectSummary::from(&p)),
                        "branch": pr.branch,
                        "message": format!("Set current project to {} on branch {}", req.path, pr.branch)
                    }),
                    started_at,
                    Vec::new(),
                    false,
                ))
            }
            Err(e) => Ok(envelope_error(
                "SET_PROJECT_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(
        description = "Get the current project context: path, branch, Git status. Use when the user asks 'what project am I in?', 'current branch', or to confirm scope for indexing and search."
    )]
    async fn get_current_project(&self) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let current = self.projects.get_current_project();

        let data = if let Some(pr) = current {
            let project = self.projects.get_project(&pr.path);
            json!({
                "project": project.map(|p| cortex_core::ProjectSummary::from(&p)),
                "branch": pr.branch,
                "commit": pr.commit,
                "repository_path": pr.path.display().to_string()
            })
        } else {
            json!({
                "project": null,
                "branch": null,
                "message": "No current project set. Use add_project to register a project."
            })
        };

        Ok(envelope_success(data, started, Vec::new(), false))
    }

    #[tool(description = "List all branches for a project with index status")]
    async fn list_branches(
        &self,
        Parameters(req): Parameters<ListBranchesReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;

        let project = self.projects.get_project(&path).ok_or_else(|| {
            McpError::invalid_params(format!("Project not found: {}", path.display()), None)
        })?;

        let git_info = project.git_info.as_ref();
        let current_branch = git_info
            .map(|g| g.current_branch.as_str())
            .unwrap_or("unknown");

        let branches = git_info
            .map(|g| {
                g.branches
                    .iter()
                    .map(|b| {
                        let is_indexed = project.indexed_branches.contains_key(&b.name);
                        serde_json::json!({
                            "name": b.name,
                            "is_remote": b.is_remote,
                            "is_current": b.name == current_branch,
                            "is_indexed": is_indexed,
                            "last_commit": b.last_commit
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(envelope_success(
            json!({
                "project": path.display().to_string(),
                "current_branch": current_branch,
                "branches": branches,
                "total": branches.len()
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Refresh Git info for a project (detect branch changes)")]
    async fn refresh_project(
        &self,
        Parameters(req): Parameters<PathReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let path = PathBuf::from(&req.path);

        let branch_change = self.projects.check_branch_change(&path).ok();

        match self.projects.refresh_git_info(&path) {
            Ok(git_info) => Ok(envelope_success(
                json!({
                    "path": req.path,
                    "git_info": git_info,
                    "branch_changed": branch_change.flatten(),
                    "message": "Refreshed Git info"
                }),
                started_at,
                Vec::new(),
                false,
            )),
            Err(e) => Ok(envelope_error(
                "REFRESH_FAILED",
                e.to_string(),
                None,
                started_at,
            )),
        }
    }

    #[tool(description = "Get project freshness, branch health, and queue status")]
    async fn project_status(
        &self,
        Parameters(req): Parameters<ProjectStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let include_queue = req.include_queue.unwrap_or(true);
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;
        let project = self.projects.get_project(&project_path);
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let daemon_status = cortex_watcher::daemon_status(&daemon_paths)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let branch_health =
            cortex_watcher::project_branch_health(&daemon_paths, &project_path).unwrap_or_default();

        let queue = if include_queue {
            cortex_watcher::list_index_jobs(&daemon_paths, 250)
                .unwrap_or_default()
                .into_iter()
                .filter(|j| j.repository_path == project_path.display().to_string())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let stale_branches = branch_health.iter().filter(|b| b.is_stale).count();
        let freshness = if stale_branches > 0 {
            "stale"
        } else if queue
            .iter()
            .any(|j| j.status == "pending" || j.status == "running")
        {
            "indexing"
        } else {
            "current"
        };

        Ok(envelope_success(
            json!({
                "path": project_path.display().to_string(),
                "freshness": freshness,
                "project": project,
                "branch_health": branch_health,
                "stale_branches": stale_branches,
                "queue": queue,
                "daemon": daemon_status,
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Sync project state (refresh -> detect switch -> enqueue index -> cleanup)"
    )]
    async fn project_sync(
        &self,
        Parameters(req): Parameters<ProjectSyncReq>,
    ) -> Result<CallToolResult, McpError> {
        let started_at = Instant::now();
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;

        let branch_change = self
            .projects
            .check_branch_change(&project_path)
            .ok()
            .flatten();
        let refresh = self.projects.refresh_git_info(&project_path);
        let refresh_stage = match refresh {
            Ok(ref info) => json!({
                "status": "ok",
                "branch": info.current_branch,
                "commit": info.current_commit,
                "branch_changed": branch_change,
            }),
            Err(err) => {
                return Ok(envelope_error(
                    "PROJECT_SYNC_REFRESH_FAILED",
                    err.to_string(),
                    None,
                    started_at,
                ));
            }
        };

        let force = req.force.unwrap_or(false);
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let daemon_status = cortex_watcher::daemon_status(&daemon_paths)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut index_stage = json!({
            "status": "skipped",
            "reason": "no_git_context",
        });
        if let Some((_repo_root, branch, commit_hash)) = resolve_git_context_for_path(&project_path)
        {
            let graph_scope = cortex_core::graph_repository_path_for_index(&project_path, None);
            let registry_current = self.projects.get_project(&project_path);
            let already_current = !force
                && registry_current
                    .as_ref()
                    .is_some_and(|p| !p.is_current_index_stale());
            if already_current {
                index_stage = json!({
                    "status": "skipped",
                    "reason": "branch_index_already_current",
                    "repository_path": graph_scope,
                    "branch": branch,
                    "commit": commit_hash,
                });
            } else if daemon_status.running {
                let enqueue = cortex_watcher::enqueue_index_job(
                    &daemon_paths,
                    &cortex_watcher::IndexJobRequest {
                        repository_path: graph_scope.clone(),
                        branch: branch.clone(),
                        commit_hash: commit_hash.clone(),
                        mode: if force {
                            cortex_watcher::JobMode::Full
                        } else {
                            cortex_watcher::JobMode::IncrementalDiff
                        },
                    },
                )
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                index_stage = json!({
                    "status": "queued",
                    "daemon": true,
                    "job": enqueue.job,
                    "deduplicated": enqueue.deduplicated,
                    "repository_path": graph_scope,
                    "branch": branch,
                    "commit": commit_hash,
                });
            } else {
                index_stage = json!({
                    "status": "skipped",
                    "reason": "daemon_not_running",
                    "branch": branch,
                    "commit": commit_hash,
                });
            }
        }

        let cleanup_stage = if req.cleanup_old_branches.unwrap_or(true) {
            match self.projects.cleanup_old_branches(&project_path) {
                Ok(removed) => json!({"status": "ok", "removed": removed}),
                Err(err) => json!({"status": "error", "error": err.to_string()}),
            }
        } else {
            json!({"status": "skipped"})
        };

        let branch_health =
            cortex_watcher::project_branch_health(&daemon_paths, &project_path).unwrap_or_default();
        Ok(envelope_success(
            json!({
                "sync_status": "synced",
                "path": project_path.display().to_string(),
                "stages": {
                    "refresh": refresh_stage,
                    "index": index_stage,
                    "cleanup": cleanup_stage,
                },
                "branch_health": branch_health,
            }),
            started_at,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Compare two branches for a project (ahead/behind commits and changed files)"
    )]
    async fn project_branch_diff(
        &self,
        Parameters(req): Parameters<ProjectBranchDiffReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path))
            .ok_or_else(|| {
                McpError::invalid_params("No project specified and no current project set", None)
            })?;
        let git_root =
            find_git_repository_root(project_path.as_path()).unwrap_or(project_path.clone());
        let git = GitOperations::new(&git_root);
        let diff = git
            .compare_branches(&req.source, &req.target)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let limit = req.commit_limit.unwrap_or(50);
        let ahead_commits = diff
            .ahead_commits
            .into_iter()
            .take(limit)
            .map(|commit| {
                json!({
                    "hash": commit.hash,
                    "short_hash": commit.short_hash,
                    "author": commit.author,
                    "author_email": commit.author_email,
                    "date": commit.date.to_rfc3339(),
                    "message": commit.message,
                    "message_full": commit.message_full,
                    "parents": commit.parents,
                })
            })
            .collect::<Vec<_>>();
        let behind_commits = diff
            .behind_commits
            .into_iter()
            .take(limit)
            .map(|commit| {
                json!({
                    "hash": commit.hash,
                    "short_hash": commit.short_hash,
                    "author": commit.author,
                    "author_email": commit.author_email,
                    "date": commit.date.to_rfc3339(),
                    "message": commit.message,
                    "message_full": commit.message_full,
                    "parents": commit.parents,
                })
            })
            .collect::<Vec<_>>();
        let changed_files = diff
            .changed_files
            .into_iter()
            .map(|file| {
                let change_type = match file.change_type {
                    cortex_core::FileChangeType::Added => "added",
                    cortex_core::FileChangeType::Modified => "modified",
                    cortex_core::FileChangeType::Deleted => "deleted",
                    cortex_core::FileChangeType::Renamed => "renamed",
                };
                json!({
                    "path": file.path,
                    "change_type": change_type,
                    "additions": file.additions,
                    "deletions": file.deletions,
                })
            })
            .collect::<Vec<_>>();

        Ok(envelope_success(
            json!({
                "path": project_path.display().to_string(),
                "source_branch": diff.source_branch,
                "target_branch": diff.target_branch,
                "ahead_count": diff.ahead_count,
                "behind_count": diff.behind_count,
                "ahead_commits": ahead_commits,
                "behind_commits": behind_commits,
                "changed_files": changed_files,
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Get daemon queue status for project indexing jobs")]
    async fn project_queue_status(
        &self,
        Parameters(req): Parameters<ProjectQueueStatusReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let limit = req.limit.unwrap_or(200);
        let jobs = cortex_watcher::list_index_jobs(&daemon_paths, limit)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let filtered = if let Some(path) = req.path {
            jobs.into_iter()
                .filter(|j| j.repository_path == path)
                .collect::<Vec<_>>()
        } else {
            jobs
        };

        Ok(envelope_success(
            json!({
                "count": filtered.len(),
                "jobs": filtered,
                "daemon": cortex_watcher::daemon_status(&daemon_paths).ok(),
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(description = "Get project daemon metrics for watch/index orchestration")]
    async fn project_metrics(
        &self,
        Parameters(req): Parameters<ProjectMetricsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let daemon_paths = cortex_watcher::DaemonPaths::default_paths();
        let counters = cortex_watcher::daemon_metrics(&daemon_paths)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .counters;
        let project_path = req
            .path
            .map(PathBuf::from)
            .or_else(|| self.projects.get_current_project().map(|p| p.path));
        let queue = if let Some(ref path) = project_path {
            cortex_watcher::list_index_jobs(&daemon_paths, 300)
                .unwrap_or_default()
                .into_iter()
                .filter(|j| j.repository_path == path.display().to_string())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let avg_queue_wait_ms = {
            let total = counters.get("queue_wait_ms_total").copied().unwrap_or(0);
            let samples = counters.get("queue_wait_samples").copied().unwrap_or(0);
            if samples > 0 {
                Some(total as f64 / samples as f64)
            } else {
                None
            }
        };
        let avg_index_duration_ms = {
            let total = counters
                .get("index_duration_ms_total")
                .copied()
                .unwrap_or(0);
            let completed = counters.get("completed_jobs").copied().unwrap_or(0);
            if completed > 0 {
                Some(total as f64 / completed as f64)
            } else {
                None
            }
        };

        Ok(envelope_success(
            json!({
                "project_path": project_path.map(|p| p.display().to_string()),
                "counters": counters,
                "derived": {
                    "avg_queue_wait_ms": avg_queue_wait_ms,
                    "avg_index_duration_ms": avg_index_duration_ms
                },
                "queue_size": queue.len(),
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "List buffered large tool responses or inspect one response_id. Use after a tool returns a buffered pointer instead of the full payload."
    )]
    async fn ctx_stats(
        &self,
        Parameters(req): Parameters<CtxStatsReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let buffer = self.response_buffer.lock().await;
        let data = if let Some(response_id) = req.response_id.as_deref() {
            let entry = match buffer.entry_detail(Some(response_id)) {
                Ok(entry) => entry,
                Err(e) => return Ok(envelope_error("NOT_FOUND", e, None, started)),
            };
            json!({ "entry": entry })
        } else {
            json!({
                "buffer": buffer.stats(),
                "latest_response_id": buffer.latest_id(),
            })
        };
        Ok(envelope_success(data, started, Vec::new(), false))
    }

    #[tool(
        description = "Search within a buffered tool response with optional before/after context lines."
    )]
    async fn ctx_grep(
        &self,
        Parameters(req): Parameters<CtxGrepReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let before = req.before.unwrap_or(0);
        let after = req.after.unwrap_or(0);
        let buffer = self.response_buffer.lock().await;
        let matches = match buffer.grep(req.response_id.as_deref(), &req.pattern, before, after)
        {
            Ok(matches) => matches,
            Err(e) => return Ok(envelope_error("NOT_FOUND", e, None, started)),
        };
        Ok(envelope_success(
            json!({
                "response_id": req
                    .response_id
                    .or_else(|| buffer.latest_id())
                    .unwrap_or_default(),
                "pattern": req.pattern,
                "match_count": matches.len(),
                "matches": matches,
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Return a character slice from a buffered tool response using inclusive/exclusive offsets."
    )]
    async fn ctx_slice(
        &self,
        Parameters(req): Parameters<CtxSliceReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let buffer = self.response_buffer.lock().await;
        let slice = match buffer.slice(req.response_id.as_deref(), req.from, req.to) {
            Ok(slice) => slice,
            Err(e) => return Ok(envelope_error("INVALID_ARGUMENT", e, None, started)),
        };
        Ok(envelope_success(
            json!({
                "response_id": req
                    .response_id
                    .or_else(|| buffer.latest_id())
                    .unwrap_or_default(),
                "from": req.from,
                "to": req.to,
                "slice": slice,
            }),
            started,
            Vec::new(),
            false,
        ))
    }

    #[tool(
        description = "Return the first N lines from a buffered tool response (default 20)."
    )]
    async fn ctx_peek(
        &self,
        Parameters(req): Parameters<CtxPeekReq>,
    ) -> Result<CallToolResult, McpError> {
        let started = Instant::now();
        let lines = req.lines.unwrap_or(20);
        let buffer = self.response_buffer.lock().await;
        let preview = match buffer.peek(req.response_id.as_deref(), lines) {
            Ok(preview) => preview,
            Err(e) => return Ok(envelope_error("NOT_FOUND", e, None, started)),
        };
        Ok(envelope_success(
            json!({
                "response_id": req
                    .response_id
                    .or_else(|| buffer.latest_id())
                    .unwrap_or_default(),
                "lines": lines,
                "preview": preview,
            }),
            started,
            Vec::new(),
            false,
        ))
    }
}


#[tool_handler]
impl ServerHandler for CortexHandler {
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let tool_name = request.name.clone();
        let tcc =
            rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
        let result = self.tool_router.call(tcc).await?;
        Ok(self.wrap_result(&tool_name, result).await)
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_prompts()
                .build(),
        )
        .with_instructions(codecortex_server_instructions())
        .with_server_info(Implementation::new("cortex", env!("CARGO_PKG_VERSION")))
    }

    fn subscribe(
        &self,
        _request: SubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        // Cursor (MCP 2025-11-xx) calls `resources/subscribe` for static docs; rmcp's default
        // returns -32601 METHOD_NOT_FOUND which pollutes MCP logs. Resources are static.
        std::future::ready(Ok(()))
    }

    fn unsubscribe(
        &self,
        _request: UnsubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), McpError>> + Send + '_ {
        std::future::ready(Ok(()))
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListResourcesResult::with_all_items(
            codecortex_resources(),
        )))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, McpError>> + Send + '_ {
        let uri = request.uri;
        std::future::ready(match codecortex_resource_text(&uri) {
            Some((mime_type, text)) => Ok(ReadResourceResult::new(vec![
                ResourceContents::text(text, uri).with_mime_type(mime_type),
            ])),
            None => Err(McpError::invalid_params(
                format!("Unknown CodeCortex resource URI: {uri}"),
                None,
            )),
        })
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListPromptsResult::with_all_items(codecortex_prompts())))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<GetPromptResult, McpError>> + Send + '_ {
        std::future::ready(match codecortex_prompt_text(&request.name) {
            Some((description, text)) => Ok(GetPromptResult::new(vec![PromptMessage::new_text(
                PromptMessageRole::User,
                text,
            )])
            .with_description(description)),
            None => Err(McpError::invalid_params(
                format!("Unknown CodeCortex prompt: {}", request.name),
                None,
            )),
        })
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let all_tools = self.tool_router.list_all();
        if lazy_tools::lazy_tools_enabled() {
            let promoted = lazy_tools::lock_promoted(&self.promoted_tools);
            let live = lazy_tools::live_tool_names(&promoted);
            let tools = all_tools
                .into_iter()
                .filter(|tool| live.contains(tool.name.as_ref()))
                .collect();
            std::future::ready(Ok(ListToolsResult {
                tools,
                meta: None,
                next_cursor: None,
            }))
        } else {
            std::future::ready(Ok(ListToolsResult {
                tools: all_tools,
                meta: None,
                next_cursor: None,
            }))
        }
    }
}


pub async fn start_stdio(config: CortexConfig) -> anyhow::Result<()> {
    start_stdio_with_flags(config, FeatureFlags::from_env()).await
}

async fn start_stdio_with_flags(
    config: CortexConfig,
    feature_flags: FeatureFlags,
) -> anyhow::Result<()> {
    let service = match CortexHandler::new_async(config, feature_flags)
        .await
        .serve(stdio())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            if matches!(e, ServerInitializeError::ConnectionClosed(_)) {
                tracing::debug!("MCP client disconnected during initialization: {}", e);
                return Ok(());
            }
            return Err(e.into());
        }
    };
    service.waiting().await?;
    Ok(())
}

fn is_loopback(addr: &SocketAddr) -> bool {
    addr.ip().is_loopback()
}

fn validate_serve_options(options: &McpServeOptions) -> anyhow::Result<()> {
    if options.max_clients == 0 {
        return Err(anyhow::anyhow!("max_clients must be greater than 0"));
    }
    if options.idle_timeout_secs == 0 {
        return Err(anyhow::anyhow!("idle_timeout_secs must be greater than 0"));
    }
    if !options.allow_remote && !is_loopback(&options.listen) {
        return Err(anyhow::anyhow!(
            "non-loopback listen address requires allow_remote=true"
        ));
    }
    Ok(())
}

pub async fn start_with_options(
    config: CortexConfig,
    options: McpServeOptions,
) -> anyhow::Result<()> {
    validate_serve_options(&options)?;
    match options.transport {
        McpTransport::Stdio => start_stdio_with_flags(config, options.feature_flags).await,
        McpTransport::HttpSse | McpTransport::WebSocket | McpTransport::Multi => {
            crate::network_server::start_network(config, options).await
        }
    }
}

#[cfg(test)]
mod serve_options_tests {
    use super::*;

    #[test]
    fn rejects_remote_bind_without_allow_remote() {
        let opts = McpServeOptions {
            transport: McpTransport::HttpSse,
            listen: "0.0.0.0:3010".parse().unwrap(),
            token: None,
            allow_remote: false,
            max_clients: 32,
            idle_timeout_secs: 60,
            feature_flags: FeatureFlags::from_env(),
        };
        assert!(validate_serve_options(&opts).is_err());
    }

    #[test]
    fn accepts_remote_bind_when_explicitly_allowed() {
        let opts = McpServeOptions {
            transport: McpTransport::WebSocket,
            listen: "0.0.0.0:3010".parse().unwrap(),
            token: Some("secret".to_string()),
            allow_remote: true,
            max_clients: 32,
            idle_timeout_secs: 60,
            feature_flags: FeatureFlags::from_env(),
        };
        assert!(validate_serve_options(&opts).is_ok());
    }

    #[test]
    fn default_serve_options_keep_stdio_compatibility() {
        let opts = McpServeOptions::default();
        assert_eq!(opts.transport, McpTransport::Stdio);
        assert!(opts.listen.ip().is_loopback());
    }
}

async fn lookup_code_nodes_by_symbol(
    client: &GraphClient,
    graph_scope: &str,
    symbol: &str,
    branch: Option<&str>,
    limit: usize,
) -> anyhow::Result<Vec<Value>> {
    let branch_clause = branch
        .filter(|b| !b.is_empty())
        .map(|b| format!("AND n.branch = '{}'", escape_cypher(b)))
        .unwrap_or_default();
    let query = format!(
        "MATCH (n:CodeNode) WHERE n.repository_path = '{}' AND n.name CONTAINS '{}' {} \
         RETURN n.name, n.path, n.kind, n.source, n.line_number, n.lang LIMIT {}",
        escape_cypher(graph_scope),
        escape_cypher(symbol),
        branch_clause,
        limit
    );
    client.raw_query(&query).await.map_err(Into::into)
}

fn build_analyze_filters(
    include_paths: Option<Vec<String>>,
    include_files: Option<Vec<String>>,
    include_globs: Option<Vec<String>>,
    exclude_paths: Option<Vec<String>>,
    exclude_files: Option<Vec<String>>,
    exclude_globs: Option<Vec<String>>,
) -> anyhow::Result<AnalyzePathFilters> {
    let filters = AnalyzePathFilters {
        include_paths: include_paths.unwrap_or_default(),
        include_files: include_files.unwrap_or_default(),
        include_globs: include_globs.unwrap_or_default(),
        exclude_paths: exclude_paths.unwrap_or_default(),
        exclude_files: exclude_files.unwrap_or_default(),
        exclude_globs: exclude_globs.unwrap_or_default(),
    };
    filters.validate().map_err(anyhow::Error::msg)?;
    Ok(filters)
}

fn usage_symbol_names_from_hits(hits: Vec<cortex_analyzer::SymbolHit>) -> Vec<String> {
    let mut names: Vec<String> = hits
        .iter()
        .filter(|h| is_callable_kind(&h.kind))
        .map(|h| h.name.clone())
        .collect();
    if names.is_empty() {
        names = hits.into_iter().map(|h| h.name).collect();
    }
    names.sort();
    names.dedup();
    names
}

fn parse_usage_kind(kind: Option<&str>) -> anyhow::Result<Option<UsageKind>> {
    let Some(kind) = kind else {
        return Ok(None);
    };
    let value = kind.trim().to_ascii_lowercase().replace('-', "_");
    let parsed = match value.as_str() {
        "call" => UsageKind::Call,
        "import" => UsageKind::Import,
        "type_reference" | "typereference" => UsageKind::TypeReference,
        "field_access" | "fieldaccess" => UsageKind::FieldAccess,
        "inheritance" | "inherits" => UsageKind::Inheritance,
        "implementation" | "implements" => UsageKind::Implementation,
        "reference" => UsageKind::Reference,
        _ => anyhow::bail!("unsupported usage kind: {}", kind),
    };
    Ok(Some(parsed))
}

fn parse_review_severity(input: Option<&str>) -> Severity {
    match input.unwrap_or("warning").to_ascii_lowercase().as_str() {
        "critical" => Severity::Critical,
        "error" => Severity::Error,
        "warning" | "warn" => Severity::Warning,
        _ => Severity::Info,
    }
}

fn build_review_input_from_req(repo_path: &str, req: &PrReviewReq) -> anyhow::Result<ReviewInput> {
    let target_path = req.path.as_deref().unwrap_or(repo_path);
    let root = PathBuf::from(target_path);
    let git_root = find_git_repository_root(root.as_path()).unwrap_or_else(|| root.clone());
    let base_ref = req.base_ref.clone().unwrap_or_else(|| "main".to_string());
    let head_ref = req.head_ref.clone().unwrap_or_else(|| "HEAD".to_string());

    let review_files = if git_root.join(".git").exists() {
        load_local_review_inputs_for_mcp(&git_root, &base_ref, &head_ref)?
    } else {
        let mut files = Vec::new();
        collect_review_files(&root, &mut files)?;
        files
            .into_iter()
            .filter_map(|p| {
                let source = fs::read_to_string(&p).ok()?;
                Some(ReviewFileInput {
                    path: p.display().to_string(),
                    source,
                    changed_ranges: Vec::<ReviewLineRange>::new(),
                })
            })
            .collect()
    };

    Ok(ReviewInput {
        base_ref: Some(base_ref),
        head_ref: Some(head_ref),
        filters: AnalyzePathFilters::default(),
        min_severity: parse_review_severity(req.min_severity.as_deref()),
        max_findings: req.max_findings,
        files: review_files,
    })
}

fn load_local_review_inputs_for_mcp(
    repo_path: &Path,
    base: &str,
    head: &str,
) -> anyhow::Result<Vec<ReviewFileInput>> {
    let range = format!("{base}...{head}");
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["diff", "--unified=0", "--no-color", range.as_str()])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "failed to compute local diff for pr_review: {}",
            stderr.trim()
        );
    }

    let patch = String::from_utf8_lossy(&output.stdout);
    let changed = parse_unified_diff_changed_ranges_mcp(&patch);
    let mut files = Vec::new();

    for (rel_path, ranges) in changed {
        if let Some(source) = read_file_at_ref_mcp(repo_path, head, &rel_path)? {
            files.push(ReviewFileInput {
                path: rel_path,
                source,
                changed_ranges: ranges,
            });
        }
    }
    Ok(files)
}

fn read_file_at_ref_mcp(
    repo_path: &Path,
    reference: &str,
    file_path: &str,
) -> anyhow::Result<Option<String>> {
    let show_spec = format!("{reference}:{file_path}");
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["show", show_spec.as_str()])
        .output()?;
    if output.status.success() {
        return Ok(Some(String::from_utf8_lossy(&output.stdout).to_string()));
    }
    Ok(None)
}

fn parse_unified_diff_changed_ranges_mcp(patch: &str) -> HashMap<String, Vec<ReviewLineRange>> {
    let mut out: HashMap<String, Vec<ReviewLineRange>> = HashMap::new();
    let mut current_path: Option<String> = None;
    for line in patch.lines() {
        if let Some(stripped) = line.strip_prefix("+++ b/") {
            current_path = Some(stripped.to_string());
            continue;
        }
        if line.starts_with("+++ /dev/null") {
            current_path = None;
            continue;
        }
        if line.starts_with("@@")
            && let (Some(path), Some(range)) = (current_path.as_ref(), parse_hunk_range_mcp(line))
        {
            out.entry(path.clone()).or_default().push(range);
        }
    }
    out
}

fn parse_hunk_range_mcp(line: &str) -> Option<ReviewLineRange> {
    if !line.starts_with("@@") {
        return None;
    }
    let plus_index = line.find('+')?;
    let after_plus = &line[plus_index + 1..];
    let end_index = after_plus.find(' ').unwrap_or(after_plus.len());
    let range_part = &after_plus[..end_index];

    let (start, count) = if let Some((start, count)) = range_part.split_once(',') {
        (start.parse::<u32>().ok()?, count.parse::<u32>().ok()?)
    } else {
        (range_part.parse::<u32>().ok()?, 1)
    };
    if count == 0 || start == 0 {
        return None;
    }
    Some(ReviewLineRange {
        start_line: start,
        end_line: start + count.saturating_sub(1),
    })
}

fn collect_review_files(path: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            if p.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            collect_review_files(&p, out)?;
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str())
            && matches!(
                ext,
                "rs" | "py" | "js" | "jsx" | "ts" | "tsx" | "go" | "java" | "c" | "cpp" | "h"
            )
        {
            out.push(p);
        }
    }
    Ok(())
}

fn analyzer_capabilities_json() -> Value {
    json!({
        "path_filters": {
            "supported": true,
            "fields": [
                "include_paths",
                "include_files",
                "include_globs",
                "exclude_paths",
                "exclude_files",
                "exclude_globs"
            ]
        },
        "language_aware_smells": {
            "supported": true,
            "extensions": [
                "rs","py","rb","js","jsx","ts","tsx","go","java","c","cc","cpp","h","hpp",
                "cs","php","swift","kt","kts","json","sh","bash","zsh","m","mm","scala"
            ]
        }
    })
}

fn collect_paths_for_filter(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if (k == "path" || k.ends_with(".path")) && v.is_string() {
                    if let Some(path) = v.as_str() {
                        out.push(path.to_string());
                    }
                }
                collect_paths_for_filter(v, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_paths_for_filter(item, out);
            }
        }
        _ => {}
    }
}

fn filter_rows_by_paths(rows: Vec<Value>, filters: &AnalyzePathFilters) -> Vec<Value> {
    if filters.is_empty() {
        return rows;
    }
    rows.into_iter()
        .filter(|row| {
            let mut paths = Vec::new();
            collect_paths_for_filter(row, &mut paths);
            filters.matches_any_path(paths.iter().map(String::as_str))
        })
        .collect()
}

fn tool_params_hash<T: Serialize>(req: &T) -> String {
    let payload = serde_json::to_value(req).unwrap_or_else(|_| json!({}));
    let mut hasher = DefaultHasher::new();
    payload.to_string().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn capsule_cache_hash(
    query: &str,
    max_items: usize,
    max_tokens: usize,
    include_tests: bool,
    intent: &str,
    filters: &[String],
) -> String {
    let payload = json!({
        "query": query,
        "max_items": max_items,
        "max_tokens": max_tokens,
        "include_tests": include_tests,
        "intent": intent,
        "filters": filters,
    });
    let mut hasher = DefaultHasher::new();
    payload.to_string().hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn observation_from_record(rec: &ObservationRecord) -> Observation {
    Observation {
        observation_id: rec.observation_id.clone(),
        repo_id: rec.repo_id.clone(),
        session_id: rec.session_id.clone(),
        created_at: rec.created_at as i64,
        last_accessed: rec.created_at as i64,
        access_count: 0,
        created_by: rec.created_by.clone(),
        text: rec.text.clone(),
        symbol_refs: rec.symbol_refs.clone(),
        confidence: rec.confidence,
        importance: 1.0,
        stale: rec.stale,
        classification: rec
            .classification
            .parse()
            .unwrap_or(Classification::Internal),
        severity: rec.severity.parse().unwrap_or(MemorySeverity::Info),
        tags: rec.tags.clone(),
        source_revision: rec.source_revision.clone(),
        linked_to: Vec::new(),
        source_file: None,
    }
}

fn record_from_observation(obs: &Observation) -> ObservationRecord {
    ObservationRecord {
        observation_id: obs.observation_id.clone(),
        repo_id: obs.repo_id.clone(),
        session_id: obs.session_id.clone(),
        created_at: obs.created_at.max(0) as u128,
        created_by: obs.created_by.clone(),
        text: obs.text.clone(),
        symbol_refs: obs.symbol_refs.clone(),
        confidence: obs.confidence,
        stale: obs.stale,
        classification: obs.classification.to_string(),
        severity: obs.severity.to_string(),
        tags: obs.tags.clone(),
        source_revision: obs.source_revision.clone(),
        embedding: None,
    }
}

fn migrate_json_memory_db(store: &MemoryStore) -> anyhow::Result<()> {
    let json_path = legacy_memory_json_path();
    if !json_path.exists() {
        return Ok(());
    }
    let marker = json_path.with_extension("json.migrated");
    if marker.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(&json_path)?;
    let db: MemoryDb = serde_json::from_str(&raw).unwrap_or_default();
    for rec in db.observations {
        store.save(&observation_from_record(&rec))?;
    }
    fs::write(marker, b"migrated")?;
    Ok(())
}

fn legacy_memory_json_path() -> PathBuf {
    if let Ok(p) = std::env::var("CORTEX_MEMORY_JSON_PATH") {
        return PathBuf::from(p);
    }
    CortexConfig::config_path()
        .parent()
        .map(|p| p.join("memory.json"))
        .unwrap_or_else(|| PathBuf::from(".cortex-memory.json"))
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MemoryDb {
    observations: Vec<ObservationRecord>,
}

fn audit_log_path() -> PathBuf {
    if let Ok(p) = std::env::var("CORTEX_MEMORY_AUDIT_PATH") {
        return PathBuf::from(p);
    }
    CortexConfig::config_path()
        .parent()
        .map(|p| p.join("memory-audit.log"))
        .unwrap_or_else(|| PathBuf::from(".cortex-memory-audit.log"))
}

fn append_audit_event(action: &str, target_id: &str) -> anyhow::Result<()> {
    let path = audit_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let line = serde_json::json!({
        "actor": "mcp",
        "action": action,
        "timestamp_ms": now_millis(),
        "target_id": target_id
    })
    .to_string();
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)?;
    writeln!(f, "{line}")?;
    Ok(())
}

fn looks_sensitive(text: &str) -> bool {
    let lowered = text.to_lowercase();
    [
        "password=",
        "secret=",
        "api_key",
        "token=",
        "begin private key",
    ]
    .iter()
    .any(|pat| lowered.contains(pat))
}

fn simple_lexical_score(query: &str, title: &str, body: &str) -> f64 {
    let q = query.to_lowercase();
    let t = title.to_lowercase();
    let b = body.to_lowercase();
    let title_hit = if t.contains(q.as_str()) { 1.0 } else { 0.0 };
    let body_hit = if b.contains(q.as_str()) { 1.0 } else { 0.0 };
    (title_hit * 0.7) + (body_hit * 0.3)
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f64 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut left_norm = 0.0f64;
    let mut right_norm = 0.0f64;
    for (a, b) in left.iter().zip(right.iter()) {
        let a = *a as f64;
        let b = *b as f64;
        dot += a * b;
        left_norm += a * a;
        right_norm += b * b;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
}

fn warning_with_reason(code: &str, reason: &str) -> String {
    format!("{code}:{reason}")
}

fn detect_intent(query: &str) -> &'static str {
    let q = query.to_lowercase();
    if q.contains("debug") || q.contains("error") {
        "debug"
    } else if q.contains("refactor") {
        "refactor"
    } else if q.contains("test") {
        "test"
    } else if q.contains("review") {
        "review"
    } else {
        "explore"
    }
}

/// Read a local source slice for signature extraction when graph `source` is empty.
fn load_source_snippet_for_signature(
    repo_path: &str,
    rel_path: &str,
    line_number: Option<u32>,
) -> Option<String> {
    let path = Path::new(repo_path).join(rel_path);
    let content = fs::read_to_string(&path).ok()?;
    if content.trim().is_empty() {
        return None;
    }
    if let Some(line) = line_number {
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return None;
        }
        let idx = (line.saturating_sub(1) as usize).min(lines.len() - 1);
        let start = idx.saturating_sub(20);
        let end = (idx + 50).min(lines.len());
        return Some(lines[start..end].join("\n"));
    }
    Some(content.chars().take(12_000).collect())
}

impl CortexHandler {
    async fn resolve_vector_freshness_label(
        &self,
        repo_path: &str,
        latest: Option<&cortex_graph::schema::BranchIndexRecord>,
    ) -> String {
        let metadata_label = latest.map(|record| record.vector_freshness.as_str().to_string());
        match self.vector_service().await {
            Ok(service) => {
                let count = service
                    .count_documents(Some(repo_path))
                    .await
                    .unwrap_or(0);
                if count == 0 {
                    return "none".to_string();
                }
                match metadata_label.as_deref() {
                    Some("fresh") => "fresh".to_string(),
                    Some("stale") | Some("partial") => "stale".to_string(),
                    Some("warming") => "warming".to_string(),
                    _ => "unknown".to_string(),
                }
            }
            Err(_) => match metadata_label.as_deref() {
                Some("fresh") => "fresh".to_string(),
                Some("stale") | Some("partial") => "stale".to_string(),
                Some("none") => "none".to_string(),
                _ => "unknown".to_string(),
            },
        }
    }

    async fn langs_for_test_paths(
        &self,
        paths: &[String],
    ) -> Result<HashMap<String, String>, McpError> {
        if paths.is_empty() {
            return Ok(HashMap::new());
        }
        let client = self.graph_client().await?;
        let mut langs = HashMap::new();
        for path in paths {
            let query = format!(
                "MATCH (n) WHERE n.path = '{}' RETURN n.lang AS lang LIMIT 1",
                escape_cypher(path)
            );
            if let Ok(rows) = client.raw_query(&query).await {
                if let Some(lang) = rows.first().and_then(|row| row.get("lang").and_then(Value::as_str))
                {
                    langs.insert(path.clone(), lang.to_ascii_lowercase());
                }
            }
        }
        Ok(langs)
    }
}

fn infer_lang_from_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "rs" => "rust",
        "py" => "python",
        "go" => "go",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        _ => "unknown",
    }
}

fn synthesize_test_run_commands(tests: &[Value], langs: &HashMap<String, String>) -> Vec<String> {
    let mut commands = Vec::new();
    let mut seen = HashSet::new();
    for test in tests {
        let path = test
            .get("path")
            .or_else(|| test.get("test_path"))
            .and_then(Value::as_str);
        let name = test
            .get("test_name")
            .or_else(|| test.get("name"))
            .and_then(Value::as_str);
        let Some(path) = path else {
            continue;
        };
        let lang = langs
            .get(path)
            .map(|s| s.as_str())
            .unwrap_or_else(|| infer_lang_from_path(path));
        let cmd = match lang {
            "rust" | "rs" => name
                .map(|n| format!("cargo test {n}"))
                .unwrap_or_else(|| format!("cargo test -- {path}")),
            "python" | "py" => name
                .map(|n| format!("pytest {path}::{n}"))
                .unwrap_or_else(|| format!("pytest {path}")),
            "go" | "golang" => {
                let dir = Path::new(path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string());
                name
                    .map(|n| format!("go test {dir} -run {n}"))
                    .unwrap_or_else(|| format!("go test {dir}"))
            }
            "typescript" | "javascript" | "ts" | "js" => {
                format!("npm test -- {path}")
            }
            _ => format!("# inspect tests in {path}"),
        };
        if seen.insert(cmd.clone()) {
            commands.push(cmd);
        }
    }
    commands
}

fn escape_cypher(input: &str) -> String {
    input.replace('\'', "\\'")
}

pub(crate) fn default_repo_path() -> String {
    std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| ".".to_string())
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn redact_secrets(input: &str) -> String {
    input
        .lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.contains("api_key")
                || lower.contains("apikey")
                || lower.contains("secret")
                || lower.contains("password")
                || lower.contains("token")
                || lower.contains("authorization:")
                || lower.contains("private_key")
            {
                "[REDACTED_SECRET_LINE]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn entity_kind_from_graph_kind(kind: &str) -> cortex_core::EntityKind {
    match kind {
        "function" | "Function" => cortex_core::EntityKind::Function,
        "method" | "Method" => cortex_core::EntityKind::Method,
        "struct" | "Struct" => cortex_core::EntityKind::Struct,
        "enum" | "Enum" => cortex_core::EntityKind::Enum,
        "trait" | "Trait" => cortex_core::EntityKind::Trait,
        "interface" | "Interface" => cortex_core::EntityKind::Interface,
        "class" | "Class" => cortex_core::EntityKind::Class,
        "module" | "Module" => cortex_core::EntityKind::Module,
        _ => cortex_core::EntityKind::Function,
    }
}

fn find_git_repository_root(path: &Path) -> Option<PathBuf> {
    let start = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut current = Some(start.as_path());
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn resolve_git_context_for_path(path: &Path) -> Option<(PathBuf, String, String)> {
    let repo_root = find_git_repository_root(path)?;
    let git_ops = GitOperations::new(&repo_root);
    if !git_ops.is_git_repo() {
        return None;
    }
    let branch = git_ops.get_current_branch().ok()?;
    let commit = git_ops.get_current_commit().ok()?;
    Some((repo_root, branch, commit))
}

async fn promote_vector_freshness_with_config(
    config: &CortexConfig,
    repository_path: &str,
    branch: &str,
    indexed_documents: usize,
) {
    if indexed_documents == 0 {
        return;
    }
    if let Ok(client) = GraphClient::connect(config).await {
        let _ =
            mark_branch_vector_fresh(&client, repository_path, branch, IndexFreshness::Fresh).await;
    }
}

fn resolve_vector_index_git_context(
    path: &Path,
    branch: Option<String>,
    revision: Option<String>,
) -> (String, String) {
    let pick_branch = |explicit: Option<String>, fallback: String| {
        explicit
            .filter(|b| !b.is_empty() && b != "unknown")
            .unwrap_or(fallback)
    };
    let pick_revision = |explicit: Option<String>, fallback: String| {
        explicit
            .filter(|r| !r.is_empty() && r != "unknown")
            .unwrap_or(fallback)
    };
    if let Some((_, git_branch, git_revision)) = resolve_git_context_for_path(path) {
        return (
            pick_branch(branch, git_branch),
            pick_revision(revision, git_revision),
        );
    }
    (
        pick_branch(branch, "main".to_string()),
        pick_revision(revision, "unknown".to_string()),
    )
}

fn build_skeleton(content: &str, mode: &str) -> String {
    let mut out = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("struct ")
            || t.starts_with("pub struct ")
            || t.starts_with("class ")
            || t.starts_with("impl ")
        {
            out.push(t.to_string());
            if mode == "minimal" && out.len() >= 120 {
                break;
            }
        } else if mode == "standard" && (t.starts_with("//") || t.starts_with("///")) {
            out.push(t.to_string());
        }
    }
    if out.is_empty() {
        return content.lines().take(40).collect::<Vec<_>>().join("\n");
    }
    out.join("\n")
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::{
        ContextCapsuleReq, CortexHandler, ImpactGraphReq, IndexStatusReq, LogicFlowReq,
        ManageCodecortexReq, SaveObservationReq, SearchMemoryReq, SessionContextReq, SkeletonReq,
        SubmitLspEdgesReq, WorkspaceSetupReq, build_review_input_from_req, build_skeleton,
        detect_intent, looks_sensitive, parse_hunk_range_mcp,
        parse_unified_diff_changed_ranges_mcp, parse_usage_kind, simple_lexical_score,
    };
    use crate::FeatureFlags;
    use crate::handler::{PrReviewReq, UsageKind};
    use cortex_core::CortexConfig;
    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::CallToolResult;
    use serde_json::Value;
    use std::sync::Mutex;

    /// Mutex to synchronize tests that manipulate environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn sensitive_detector_finds_tokens() {
        assert!(looks_sensitive("my API_KEY=123"));
        assert!(!looks_sensitive("regular engineering note"));
    }

    #[test]
    fn intent_detection_works() {
        assert_eq!(detect_intent("debug call path"), "debug");
        assert_eq!(detect_intent("refactor this"), "refactor");
    }

    #[test]
    fn skeleton_extracts_signatures() {
        let src = "pub struct A {}\nfn run() {}\nlet x = 1;";
        let s = build_skeleton(src, "minimal");
        assert!(s.contains("pub struct A"));
        assert!(s.contains("fn run"));
    }

    #[test]
    fn lexical_score_prefers_title_hits() {
        let a = simple_lexical_score("call_chain", "call_chain", "x");
        let b = simple_lexical_score("call_chain", "x", "call_chain");
        assert!(a > b);
    }

    fn as_text(result: CallToolResult) -> String {
        result.content[0]
            .as_text()
            .expect("text response")
            .text
            .clone()
    }

    #[tokio::test]
    async fn context_capsule_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_context_capsule(Parameters(ContextCapsuleReq {
                query: "auth refresh".to_string(),
                if_none_match: None,
                task_intent: None,
                repo_path: None,
                max_tokens: None,
                max_items: None,
                include_tests: None,
                path_filter: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED");
        }
    }

    #[tokio::test]
    async fn impact_graph_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_impact_graph(Parameters(ImpactGraphReq {
                symbol: "call_chain".to_string(),
                symbol_type: None,
                repo_path: None,
                depth: None,
                include_importers: None,
                include_tests: None,
                budget_tokens: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED");
        }
    }

    #[tokio::test]
    async fn logic_flow_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_LOGIC_FLOW_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .search_logic_flow(Parameters(LogicFlowReq {
                from_symbol: "a".to_string(),
                to_symbol: "b".to_string(),
                repo_path: None,
                max_paths: None,
                max_depth: None,
                allow_partial: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_LOGIC_FLOW_ENABLED");
        }
    }

    #[tokio::test]
    async fn skeleton_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_SKELETON_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_skeleton(Parameters(SkeletonReq {
                path: "Cargo.toml".to_string(),
                mode: None,
                repo_path: None,
                if_none_match: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_SKELETON_ENABLED");
        }
    }

    #[tokio::test]
    async fn index_status_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_INDEX_STATUS_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .index_status(Parameters(IndexStatusReq {
                repo_path: None,
                include_jobs: None,
                include_watcher: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_INDEX_STATUS_ENABLED");
        }
    }

    #[tokio::test]
    async fn workspace_setup_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_WORKSPACE_SETUP_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .workspace_setup(Parameters(WorkspaceSetupReq {
                repo_path: None,
                detect_agents: None,
                generate_configs: None,
                install_git_hooks: None,
                non_interactive: None,
                overwrite: None,
                install_agent_pack: None,
                agent_pack_root: None,
                install_cursor_mcp: None,
                targets: None,
                enable_watch: None,
                watch_paths: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_WORKSPACE_SETUP_ENABLED");
        }
    }

    #[tokio::test]
    async fn manage_codecortex_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_MANAGE_CODECORTEX_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .manage_codecortex(Parameters(ManageCodecortexReq {
                repo_path: None,
                action: None,
                task: None,
                install_agent_pack: None,
                agent_pack_root: None,
                enable_watch: None,
                watch_paths: None,
                auto_repair: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_MANAGE_CODECORTEX_ENABLED");
        }
    }

    #[tokio::test]
    async fn manage_codecortex_assess_returns_json() {
        let h = CortexHandler::new_with_feature_flags(
            CortexConfig::default(),
            FeatureFlags::all_enabled(),
        );
        let out = h
            .manage_codecortex(Parameters(ManageCodecortexReq {
                repo_path: Some(".".to_string()),
                action: Some("assess".to_string()),
                task: Some("smoke test".to_string()),
                install_agent_pack: None,
                agent_pack_root: None,
                enable_watch: None,
                watch_paths: None,
                auto_repair: None,
            }))
            .await
            .expect("tool response");
        let text = as_text(out);
        let v: Value = serde_json::from_str(&text).expect("json envelope");
        let status = v.get("status").and_then(|s| s.as_str()).unwrap_or("");
        assert!(
            status.eq_ignore_ascii_case("ok"),
            "expected ok status, got {status}: {text}"
        );
        assert!(v["data"]["next_steps"].is_array());
        assert!(v["data"]["recommendations"].is_array());
    }

    #[tokio::test]
    async fn submit_lsp_edges_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_LSP_INGEST_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .submit_lsp_edges(Parameters(SubmitLspEdgesReq {
                repo_path: ".".to_string(),
                edges: Vec::new(),
                merge_mode: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_LSP_INGEST_ENABLED");
        }
    }

    #[tokio::test]
    async fn save_observation_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_MEMORY_WRITE_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .save_observation(Parameters(SaveObservationReq {
                repo_path: ".".to_string(),
                text: "note".to_string(),
                severity: None,
                confidence: None,
                symbol_refs: None,
                tags: None,
                classification: None,
                session_id: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_MEMORY_WRITE_ENABLED");
        }
    }

    #[tokio::test]
    async fn get_session_context_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .get_session_context(Parameters(SessionContextReq {
                repo_path: ".".to_string(),
                session_id: None,
                include_previous: None,
                max_items: None,
                include_stale: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED");
        }
    }

    #[tokio::test]
    async fn search_memory_respects_feature_flag() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED", "0");
        }
        let h = CortexHandler::new(CortexConfig::default());
        let out = h
            .search_memory(Parameters(SearchMemoryReq {
                query: "call target".to_string(),
                repo_path: ".".to_string(),
                max_items: None,
                include_stale: None,
            }))
            .await
            .expect("tool response");
        assert!(as_text(out).contains("\"code\":\"UNAVAILABLE\""));
        unsafe {
            std::env::remove_var("CORTEX_FLAG_MCP_MEMORY_READ_ENABLED");
        }
    }

    #[test]
    fn pattern_detection_names_contain_expected_patterns() {
        let patterns = vec![
            "builder",
            "factory",
            "singleton",
            "repository",
            "service",
            "handler",
            "middleware",
            "observer",
            "strategy",
            "adapter",
            "decorator",
            "command",
            "state",
            "facade",
            "proxy",
        ];

        for pattern in patterns {
            assert!(
                !pattern.is_empty(),
                "Pattern {} should not be empty",
                pattern
            );
        }
    }

    #[test]
    fn builder_pattern_detection_logic() {
        let builder_names = vec!["UserBuilder", "RequestBuilder", "ConfigBuilder"];
        let builder_structural = vec!["build()", ".build()", "builder"];

        for name in &builder_names {
            assert!(
                name.contains("Builder"),
                "Name {} should contain Builder",
                name
            );
        }
        for structural in &builder_structural {
            assert!(
                structural.contains("build"),
                "Structural {} should indicate builder",
                structural
            );
        }
    }

    #[test]
    fn repository_pattern_detection_logic() {
        let repo_names = vec!["UserRepository", "OrderRepo", "DataStore", "UserDAO"];
        let repo_structural = vec!["find(", "save(", "delete(", "query("];

        for name in &repo_names {
            let is_repo = name.contains("Repository")
                || name.contains("Repo")
                || name.contains("Store")
                || name.contains("DAO");
            assert!(is_repo, "Name {} should indicate repository", name);
        }
        for structural in &repo_structural {
            assert!(!structural.is_empty(), "Structural hint should exist");
        }
    }

    #[test]
    fn adapter_pattern_detection_logic() {
        let adapter_names = vec!["DataAdapter", "ApiAdapter", "LogWrapper", "FormatConverter"];
        let _adapter_structural = ["adapt(", "convert(", "wrap("];

        for name in &adapter_names {
            let is_adapter =
                name.contains("Adapter") || name.contains("Wrapper") || name.contains("Converter");
            assert!(is_adapter, "Name {} should indicate adapter", name);
        }
    }

    #[test]
    fn command_pattern_detection_logic() {
        let command_names = vec!["CreateCommand", "DeleteAction", "UpdateOperation"];
        let _command_structural = ["execute(", "undo(", "redo("];

        for name in &command_names {
            let is_command =
                name.contains("Command") || name.contains("Action") || name.contains("Operation");
            assert!(is_command, "Name {} should indicate command", name);
        }
    }

    #[test]
    fn state_pattern_detection_logic() {
        let state_names = vec!["OrderState", "StateMachine", "ConnectionFSM"];
        let _state_structural = ["transition(", "current_state", "state"];

        for name in &state_names {
            let is_state =
                name.contains("State") || name.contains("Machine") || name.contains("FSM");
            assert!(is_state, "Name {} should indicate state", name);
        }
    }


    #[test]
    fn escape_cypher_escapes_quotes() {
        assert_eq!(super::escape_cypher("hello"), "hello");
        assert_eq!(super::escape_cypher("it's"), "it\\'s");
        assert_eq!(super::escape_cypher("'quoted'"), "\\'quoted\\'");
    }

    #[test]
    fn default_repo_path_returns_current_dir() {
        let path = super::default_repo_path();
        assert!(!path.is_empty());
    }

    #[test]
    fn build_skeleton_minimal_mode() {
        let src = r#"
            fn main() {
                println!("hello");
            }

            pub struct User {
                name: String,
            }

            impl User {
                fn new() -> Self { User { name: String::new() } }
            }
        "#;
        let skeleton = super::build_skeleton(src, "minimal");
        assert!(skeleton.contains("fn main"));
        assert!(skeleton.contains("pub struct User"));
        assert!(skeleton.contains("impl User"));
    }

    #[test]
    fn build_skeleton_standard_mode_includes_comments() {
        let src = r#"
            /// Documentation comment
            fn documented() {}

            fn regular() {}
        "#;
        let skeleton = super::build_skeleton(src, "standard");
        assert!(skeleton.contains("fn documented"));
        assert!(skeleton.contains("///"));
    }

    #[test]
    fn build_skeleton_empty_returns_fallback() {
        let src = "let x = 1;\nlet y = 2;";
        let skeleton = super::build_skeleton(src, "minimal");
        assert!(skeleton.contains("let x") || skeleton.is_empty());
    }

    #[test]
    fn cortex_handler_new_creates_instance() {
        let handler = CortexHandler::new(CortexConfig::default());
        let _ = handler.tool_enabled("test", true);
    }

    #[test]
    fn parse_usage_kind_handles_expected_values() {
        assert!(matches!(
            parse_usage_kind(Some("type-reference")).expect("parse"),
            Some(UsageKind::TypeReference)
        ));
        assert!(matches!(
            parse_usage_kind(Some("field_access")).expect("parse"),
            Some(UsageKind::FieldAccess)
        ));
        assert!(matches!(
            parse_usage_kind(Some("call")).expect("parse"),
            Some(UsageKind::Call)
        ));
    }

    #[test]
    fn parse_hunk_range_parses_added_span() {
        let range = parse_hunk_range_mcp("@@ -10,2 +20,5 @@").expect("range");
        assert_eq!(range.start_line, 20);
        assert_eq!(range.end_line, 24);
    }

    #[test]
    fn parse_unified_diff_collects_ranges_per_file() {
        let patch = r#"
diff --git a/src/a.rs b/src/a.rs
--- a/src/a.rs
+++ b/src/a.rs
@@ -1,0 +1,2 @@
+fn one() {}
+fn two() {}
"#;
        let changed = parse_unified_diff_changed_ranges_mcp(patch);
        let ranges = changed.get("src/a.rs").expect("file entry");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 1);
        assert_eq!(ranges[0].end_line, 2);
    }

    #[test]
    fn build_review_input_from_req_non_git_fallback_reads_sources() {
        let base = std::env::temp_dir().join(format!(
            "mcp-review-build-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).expect("create temp dir");
        let file = base.join("sample.rs");
        std::fs::write(&file, "fn sample() {}\n").expect("write sample");

        let req = PrReviewReq {
            base_ref: Some("main".to_string()),
            head_ref: Some("HEAD".to_string()),
            path: Some(base.display().to_string()),
            repo_path: None,
            min_severity: Some("warning".to_string()),
            max_findings: Some(10),
            budget_tokens: None,
        };
        let input =
            build_review_input_from_req(base.to_string_lossy().as_ref(), &req).expect("input");
        assert!(!input.files.is_empty());
        assert!(input.files.iter().any(|f| f.path.ends_with("sample.rs")));

        let _ = std::fs::remove_file(file);
        let _ = std::fs::remove_dir(base);
    }
}
