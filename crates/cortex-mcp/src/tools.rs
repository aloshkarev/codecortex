use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCostClass {
    Cheap,
    Bounded,
    Expensive,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutTier {
    Short,
    Medium,
    Long,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexTier {
    None,
    Project,
    Graph,
    Vector,
    GraphAndVector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenPolicy {
    MetadataOnly,
    Bounded,
    SourceSnippets,
    UnboundedForbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ToolMetadata {
    pub name: &'static str,
    pub cost_class: ToolCostClass,
    pub timeout_tier: TimeoutTier,
    pub minimum_index_tier: IndexTier,
    pub token_policy: TokenPolicy,
    pub privacy_risk: PrivacyRisk,
    pub can_return_source: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolGuidance {
    pub name: String,
    pub summary: String,
    pub use_cases: Vec<String>,
    pub avoid_when: Vec<String>,
    pub preconditions: Vec<String>,
    pub follow_ups: Vec<String>,
    pub example: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCard {
    #[serde(flatten)]
    pub metadata: ToolMetadata,
    pub guidance: ToolGuidance,
}

const TOOL_NAMES: &[&str] = &[
    "add_code_to_graph",
    "watch_directory",
    "list_watched_paths",
    "unwatch_directory",
    "find_code",
    "analyze_code_relationships",
    "execute_cypher_query",
    "find_dead_code",
    "find_clones",
    "go_to_definition",
    "find_all_usages",
    "quick_info",
    "branch_structural_diff",
    "pr_review",
    "find_similar_across_projects",
    "find_shared_dependencies",
    "compare_api_surface",
    "calculate_cyclomatic_complexity",
    "vector_index_repository",
    "vector_index_file",
    "vector_search",
    "vector_search_hybrid",
    "search_across_projects",
    "vector_index_status",
    "vector_delete_repository",
    "get_context_capsule",
    "get_patch_context",
    "get_delta_context",
    "get_test_context",
    "get_api_contract",
    "summarize_module",
    "estimate_context_cost",
    "recommend_tools",
    "tools_search",
    "tool_profile",
    "get_tool_guidance",
    "explain_index_freshness",
    "get_impact_graph",
    "search_logic_flow",
    "get_skeleton",
    "index_status",
    "workspace_setup",
    "cortex_a2a_spawn_session",
    "cortex_a2a_get_task",
    "cortex_a2a_send_message",
    "cortex_a2a_cancel_task",
    "cortex_a2a_list_tasks",
    "cortex_a2a_subscribe_task",
    "cortex_a2a_list_push_configs",
    "manage_codecortex",
    "submit_lsp_edges",
    "save_observation",
    "get_session_context",
    "search_memory",
    "list_indexed_repositories",
    "delete_repository",
    "get_repository_stats",
    "check_job_status",
    "list_jobs",
    "load_bundle",
    "export_bundle",
    "check_health",
    "diagnose",
    "get_signature",
    "find_tests",
    "explain_result",
    "analyze_refactoring",
    "find_patterns",
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
    "ctx_stats",
    "ctx_grep",
    "ctx_slice",
    "ctx_peek",
];

const TOOL_METADATA: &[ToolMetadata] = &[
    bg("add_code_to_graph", IndexTier::None, PrivacyRisk::Medium),
    bg("watch_directory", IndexTier::None, PrivacyRisk::Low),
    cheap("list_watched_paths", IndexTier::None, false),
    cheap("unwatch_directory", IndexTier::None, false),
    bounded_source("find_code", IndexTier::Graph),
    expensive("analyze_code_relationships", IndexTier::Graph, false),
    expensive("execute_cypher_query", IndexTier::Graph, true),
    expensive("find_dead_code", IndexTier::Graph, false),
    expensive("find_clones", IndexTier::Graph, false),
    cheap("go_to_definition", IndexTier::Graph, false),
    bounded("find_all_usages", IndexTier::Graph, false),
    cheap("quick_info", IndexTier::Graph, false),
    expensive("branch_structural_diff", IndexTier::Graph, false),
    expensive("pr_review", IndexTier::Graph, true),
    expensive(
        "find_similar_across_projects",
        IndexTier::GraphAndVector,
        false,
    ),
    expensive("find_shared_dependencies", IndexTier::Graph, false),
    expensive("compare_api_surface", IndexTier::Graph, false),
    expensive("calculate_cyclomatic_complexity", IndexTier::Graph, false),
    bg(
        "vector_index_repository",
        IndexTier::None,
        PrivacyRisk::Medium,
    ),
    bg("vector_index_file", IndexTier::None, PrivacyRisk::Medium),
    bounded_source("vector_search", IndexTier::Vector),
    bounded_source("vector_search_hybrid", IndexTier::GraphAndVector),
    bounded_source("search_across_projects", IndexTier::Vector),
    cheap("vector_index_status", IndexTier::None, false),
    bg(
        "vector_delete_repository",
        IndexTier::None,
        PrivacyRisk::Low,
    ),
    bounded_source("get_context_capsule", IndexTier::GraphAndVector),
    bounded_source("get_patch_context", IndexTier::GraphAndVector),
    bounded_source("get_delta_context", IndexTier::Graph),
    bounded("get_test_context", IndexTier::Graph, false),
    bounded("get_api_contract", IndexTier::Graph, false),
    bounded_source("summarize_module", IndexTier::Graph),
    cheap("estimate_context_cost", IndexTier::None, false),
    cheap("recommend_tools", IndexTier::None, false),
    cheap("tools_search", IndexTier::None, false),
    cheap("tool_profile", IndexTier::None, false),
    cheap("get_tool_guidance", IndexTier::None, false),
    cheap("explain_index_freshness", IndexTier::None, false),
    bounded("get_impact_graph", IndexTier::Graph, false),
    bounded("search_logic_flow", IndexTier::Graph, false),
    bounded_source("get_skeleton", IndexTier::None),
    cheap("index_status", IndexTier::None, false),
    bounded("workspace_setup", IndexTier::None, false),
    bounded("manage_codecortex", IndexTier::None, false),
    bounded("cortex_a2a_spawn_session", IndexTier::Graph, false),
    cheap("cortex_a2a_get_task", IndexTier::None, false),
    bounded("cortex_a2a_send_message", IndexTier::Graph, false),
    cheap("cortex_a2a_cancel_task", IndexTier::None, false),
    cheap("cortex_a2a_list_tasks", IndexTier::None, false),
    cheap("cortex_a2a_subscribe_task", IndexTier::None, false),
    cheap("cortex_a2a_list_push_configs", IndexTier::None, false),
    bounded("submit_lsp_edges", IndexTier::Graph, false),
    bounded("save_observation", IndexTier::None, false),
    bounded("get_session_context", IndexTier::None, false),
    bounded("search_memory", IndexTier::None, false),
    cheap("list_indexed_repositories", IndexTier::None, false),
    bounded("delete_repository", IndexTier::None, false),
    cheap("get_repository_stats", IndexTier::None, false),
    cheap("check_job_status", IndexTier::None, false),
    cheap("list_jobs", IndexTier::None, false),
    bounded("load_bundle", IndexTier::None, false),
    bounded("export_bundle", IndexTier::Graph, false),
    cheap("check_health", IndexTier::None, false),
    cheap("diagnose", IndexTier::None, false),
    cheap("get_signature", IndexTier::Graph, false),
    bounded("find_tests", IndexTier::Graph, false),
    cheap("explain_result", IndexTier::None, false),
    bounded("analyze_refactoring", IndexTier::Graph, false),
    bounded("find_patterns", IndexTier::Graph, false),
    cheap("list_projects", IndexTier::Project, false),
    bounded("add_project", IndexTier::None, false),
    bounded("remove_project", IndexTier::Project, false),
    cheap("set_current_project", IndexTier::Project, false),
    cheap("get_current_project", IndexTier::Project, false),
    cheap("list_branches", IndexTier::Project, false),
    bounded("refresh_project", IndexTier::Project, false),
    cheap("project_status", IndexTier::Project, false),
    bg("project_sync", IndexTier::Project, PrivacyRisk::Low),
    bounded("project_branch_diff", IndexTier::Project, false),
    cheap("project_queue_status", IndexTier::Project, false),
    cheap("project_metrics", IndexTier::Project, false),
    cheap("ctx_stats", IndexTier::None, false),
    cheap("ctx_grep", IndexTier::None, false),
    cheap("ctx_slice", IndexTier::None, false),
    cheap("ctx_peek", IndexTier::None, false),
];

const fn cheap(
    name: &'static str,
    minimum_index_tier: IndexTier,
    can_return_source: bool,
) -> ToolMetadata {
    ToolMetadata {
        name,
        cost_class: ToolCostClass::Cheap,
        timeout_tier: TimeoutTier::Short,
        minimum_index_tier,
        token_policy: if can_return_source {
            TokenPolicy::SourceSnippets
        } else {
            TokenPolicy::MetadataOnly
        },
        privacy_risk: if can_return_source {
            PrivacyRisk::Medium
        } else {
            PrivacyRisk::Low
        },
        can_return_source,
    }
}

const fn bounded(
    name: &'static str,
    minimum_index_tier: IndexTier,
    can_return_source: bool,
) -> ToolMetadata {
    ToolMetadata {
        name,
        cost_class: ToolCostClass::Bounded,
        timeout_tier: TimeoutTier::Medium,
        minimum_index_tier,
        token_policy: if can_return_source {
            TokenPolicy::SourceSnippets
        } else {
            TokenPolicy::Bounded
        },
        privacy_risk: if can_return_source {
            PrivacyRisk::Medium
        } else {
            PrivacyRisk::Low
        },
        can_return_source,
    }
}

const fn bounded_source(name: &'static str, minimum_index_tier: IndexTier) -> ToolMetadata {
    bounded(name, minimum_index_tier, true)
}

const fn expensive(
    name: &'static str,
    minimum_index_tier: IndexTier,
    can_return_source: bool,
) -> ToolMetadata {
    ToolMetadata {
        name,
        cost_class: ToolCostClass::Expensive,
        timeout_tier: TimeoutTier::Long,
        minimum_index_tier,
        token_policy: if can_return_source {
            TokenPolicy::SourceSnippets
        } else {
            TokenPolicy::Bounded
        },
        privacy_risk: if can_return_source {
            PrivacyRisk::High
        } else {
            PrivacyRisk::Medium
        },
        can_return_source,
    }
}

const fn bg(
    name: &'static str,
    minimum_index_tier: IndexTier,
    privacy_risk: PrivacyRisk,
) -> ToolMetadata {
    ToolMetadata {
        name,
        cost_class: ToolCostClass::Background,
        timeout_tier: TimeoutTier::Background,
        minimum_index_tier,
        token_policy: TokenPolicy::MetadataOnly,
        privacy_risk,
        can_return_source: false,
    }
}

pub fn tool_names() -> &'static [&'static str] {
    TOOL_NAMES
}

pub fn tool_metadata() -> &'static [ToolMetadata] {
    TOOL_METADATA
}

pub fn tool_metadata_for(name: &str) -> Option<&'static ToolMetadata> {
    TOOL_METADATA.iter().find(|meta| meta.name == name)
}

/// Order-of-magnitude hint for typical tool JSON payload size (agent budgeting; not a hard cap).
pub fn output_token_hint(meta: &ToolMetadata) -> usize {
    let base = match meta.cost_class {
        ToolCostClass::Cheap => 400,
        ToolCostClass::Bounded => 2800,
        ToolCostClass::Expensive => 7500,
        ToolCostClass::Background => 900,
    };
    let factor = match meta.token_policy {
        TokenPolicy::MetadataOnly => 1,
        TokenPolicy::Bounded => 2,
        TokenPolicy::SourceSnippets => 3,
        TokenPolicy::UnboundedForbidden => 1,
    };
    ((base * factor) / 2).clamp(200, 16_000)
}

pub fn tool_cards() -> Vec<ToolCard> {
    TOOL_METADATA
        .iter()
        .copied()
        .map(|metadata| ToolCard {
            metadata,
            guidance: tool_guidance_for(metadata.name),
        })
        .collect()
}

pub fn tool_guidance_for(name: &str) -> ToolGuidance {
    let fallback = || ToolGuidance {
        name: name.to_string(),
        summary: "General CodeCortex MCP tool.".to_string(),
        use_cases: vec!["Use when the user explicitly asks for this operation.".to_string()],
        avoid_when: vec!["Avoid unscoped broad calls on large repositories.".to_string()],
        preconditions: vec![
            "Run check_health and index_status before relying on graph-backed conclusions."
                .to_string(),
        ],
        follow_ups: vec![
            "Use explain_result for interpretation or a narrower scoped tool for details."
                .to_string(),
        ],
        example: serde_json::json!({}),
    };

    match name {
        "check_health" => ToolGuidance {
            name: name.to_string(),
            summary: "Fast health preflight for FalkorDB/graph connectivity and analyzer capabilities."
                .to_string(),
            use_cases: vec![
                "Start every critical agent workflow here.".to_string(),
                "Use when a graph-backed tool fails or appears stale.".to_string(),
            ],
            avoid_when: vec!["Do not treat this as proof a specific repository is freshly indexed.".to_string()],
            preconditions: vec![],
            follow_ups: vec!["index_status".to_string(), "diagnose".to_string()],
            example: serde_json::json!({}),
        },
        "index_status" => ToolGuidance {
            name: name.to_string(),
            summary: "Report index, watcher, job, and repair status for a repository.".to_string(),
            use_cases: vec![
                "Verify freshness before impact, review, or patch planning.".to_string(),
                "Find exact repair commands when graph/vector state is stale or unknown.".to_string(),
            ],
            avoid_when: vec!["Avoid high-confidence conclusions when it reports stale, partial, or unknown freshness.".to_string()],
            preconditions: vec!["Provide repo_path when working outside the current directory.".to_string()],
            follow_ups: vec!["project_sync".to_string(), "explain_index_freshness".to_string()],
            example: serde_json::json!({"repo_path": "/path/to/repo", "include_jobs": true, "include_watcher": true}),
        },
        "get_patch_context" => ToolGuidance {
            name: name.to_string(),
            summary: "Best first context pack before an AI agent edits code.".to_string(),
            use_cases: vec![
                "Plan a bugfix, feature, refactor, or test update under a token budget.".to_string(),
                "Collect target symbols, contract hints, likely tests, risks, and next tools.".to_string(),
            ],
            avoid_when: vec![
                "Avoid for pure navigation questions; use quick_info or go_to_definition instead.".to_string(),
                "Avoid unscoped use on monorepos unless budget and include_paths are set.".to_string(),
            ],
            preconditions: vec![
                "Run check_health and index_status first.".to_string(),
                "Pass include_paths for the module when known.".to_string(),
            ],
            follow_ups: vec!["get_api_contract".to_string(), "get_test_context".to_string(), "get_skeleton".to_string()],
            example: serde_json::json!({"task": "add token refresh to auth client", "include_paths": ["src/auth"], "budget_tokens": 6000, "mode": "feature"}),
        },
        "recommend_tools" => ToolGuidance {
            name: name.to_string(),
            summary: "Recommend the smallest safe CodeCortex tool sequence for an agent task."
                .to_string(),
            use_cases: vec![
                "Choose tools without loading the full catalog into context.".to_string(),
                "Plan efficient preflight, context, impact, review, or repair workflows.".to_string(),
            ],
            avoid_when: vec![
                "Do not treat recommendations as execution results; call the returned tools."
                    .to_string(),
            ],
            preconditions: vec![
                "Provide the user task and known scope; include privacy/freshness constraints when known."
                    .to_string(),
            ],
            follow_ups: vec!["get_tool_guidance".to_string(), "check_health".to_string()],
            example: serde_json::json!({"task": "fix flaky auth refresh test", "intent": "patch", "include_paths": ["src/auth"], "budget_tokens": 6000, "allow_source": false}),
        },
        "get_tool_guidance" => ToolGuidance {
            name: name.to_string(),
            summary: "Fetch narrow guidance cards for one tool or for tools relevant to a task."
                .to_string(),
            use_cases: vec![
                "Inspect exact use cases, preconditions, avoid rules, and examples for a tool.".to_string(),
                "Reduce agent context compared with reading codecortex://tools/catalog.".to_string(),
            ],
            avoid_when: vec!["Use codecortex://tools/catalog when an integration needs the complete catalog.".to_string()],
            preconditions: vec!["Pass tool_name for a single card or task for filtered cards.".to_string()],
            follow_ups: vec!["recommend_tools".to_string()],
            example: serde_json::json!({"tool_name": "get_patch_context"}),
        },
        "get_context_capsule" => ToolGuidance {
            name: name.to_string(),
            summary: "General token-bounded context retrieval for reasoning.".to_string(),
            use_cases: vec![
                "Answer architecture or implementation questions with compact evidence.".to_string(),
                "Gather ranked snippets when the task is not yet a concrete patch.".to_string(),
            ],
            avoid_when: vec!["Use get_patch_context for edit planning; it is more structured.".to_string()],
            preconditions: vec!["Set max_tokens and path_filter for predictable context size.".to_string()],
            follow_ups: vec!["get_signature".to_string(), "get_skeleton".to_string(), "get_test_context".to_string()],
            example: serde_json::json!({"query": "how authentication refresh works", "max_tokens": 4000, "path_filter": ["src/auth"]}),
        },
        "get_delta_context" | "branch_structural_diff" | "pr_review" => ToolGuidance {
            name: name.to_string(),
            summary: "Branch/change review context for changed symbols and impact.".to_string(),
            use_cases: vec![
                "Review a branch before merge.".to_string(),
                "Find affected callers, renamed/removed symbols, and tests to update.".to_string(),
            ],
            avoid_when: vec!["Avoid when branches are not indexed; run project_sync first.".to_string()],
            preconditions: vec!["Graph index should exist for source and target branch.".to_string()],
            follow_ups: vec!["get_test_context".to_string(), "get_impact_graph".to_string()],
            example: serde_json::json!({"source_branch": "feature/auth-refresh", "target_branch": "main", "budget_tokens": 6000}),
        },
        "get_api_contract" | "get_signature" => ToolGuidance {
            name: name.to_string(),
            summary: "Signature and API contract lookup without full-source exposure.".to_string(),
            use_cases: vec![
                "Understand call shape before editing.".to_string(),
                "Find public API, trait/interface, parameter, and return hints.".to_string(),
            ],
            avoid_when: vec!["Use get_skeleton for file-level structure instead of symbol-level contracts.".to_string()],
            preconditions: vec!["Provide a symbol or a precise name fragment.".to_string()],
            follow_ups: vec!["find_all_usages".to_string(), "get_patch_context".to_string()],
            example: serde_json::json!({"symbol": "AuthClient", "include_related": true}),
        },
        "get_test_context" | "find_tests" => ToolGuidance {
            name: name.to_string(),
            summary: "Find tests likely affected by a symbol or change.".to_string(),
            use_cases: vec![
                "Before modifying code, identify tests to read or update.".to_string(),
                "After a patch, choose a focused validation set.".to_string(),
            ],
            avoid_when: vec!["Do not infer full coverage from naming-only matches.".to_string()],
            preconditions: vec!["Provide the target symbol name.".to_string()],
            follow_ups: vec!["get_patch_context".to_string(), "calculate_cyclomatic_complexity".to_string()],
            example: serde_json::json!({"symbol": "refresh_token", "budget_tokens": 3000}),
        },
        "vector_search" | "vector_search_hybrid" | "search_across_projects" => ToolGuidance {
            name: name.to_string(),
            summary: "Semantic retrieval for natural-language code discovery.".to_string(),
            use_cases: vec![
                "Find conceptually related code when names are unknown.".to_string(),
                "Search across projects for similar implementations.".to_string(),
            ],
            avoid_when: vec!["Avoid if vector_index_status is unhealthy; use find_code or graph tools as fallback.".to_string()],
            preconditions: vec!["Vector index must exist unless fallback behavior is acceptable.".to_string()],
            follow_ups: vec!["get_context_capsule".to_string(), "get_api_contract".to_string()],
            example: serde_json::json!({"query": "token refresh retry logic", "k": 10, "search_type": "hybrid"}),
        },
        "analyze_code_relationships" | "get_impact_graph" | "search_logic_flow" => ToolGuidance {
            name: name.to_string(),
            summary: "Graph relationship and impact analysis.".to_string(),
            use_cases: vec![
                "Find callers/callees, blast radius, and logic paths.".to_string(),
                "Support refactor planning and regression-risk analysis.".to_string(),
            ],
            avoid_when: vec!["Avoid broad unfiltered traversal on stale or very large graphs.".to_string()],
            preconditions: vec!["Graph index should be fresh; pass depth/scope filters.".to_string()],
            follow_ups: vec!["get_test_context".to_string(), "analyze_refactoring".to_string()],
            example: serde_json::json!({"symbol": "authenticate", "depth": 3, "include_tests": false}),
        },
        "execute_cypher_query" => ToolGuidance {
            name: name.to_string(),
            summary: "Advanced raw graph query escape hatch.".to_string(),
            use_cases: vec!["Use only when no typed MCP tool can answer the question.".to_string()],
            avoid_when: vec![
                "Avoid for normal search, navigation, and impact tasks.".to_string(),
                "Avoid queries returning source bodies or unbounded rows.".to_string(),
            ],
            preconditions: vec!["Keep LIMIT clauses explicit and scope by repository/path.".to_string()],
            follow_ups: vec!["explain_result".to_string()],
            example: serde_json::json!({"query": "MATCH (n:CodeNode) RETURN n.name, n.path LIMIT 20"}),
        },
        "ctx_stats" => ToolGuidance {
            name: name.to_string(),
            summary: "Inspect the in-process ring buffer of large tool responses.".to_string(),
            use_cases: vec![
                "See which response_id handles are available after a buffered tool call.".to_string(),
                "Check byte and line counts before re-cutting a large payload.".to_string(),
            ],
            avoid_when: vec![
                "Not needed for small responses that were not buffered.".to_string(),
            ],
            preconditions: vec![
                "A prior tool response must have been buffered (>=1 KiB successful envelope)."
                    .to_string(),
            ],
            follow_ups: vec!["ctx_peek".to_string(), "ctx_grep".to_string(), "ctx_slice".to_string()],
            example: serde_json::json!({}),
        },
        "ctx_grep" | "ctx_slice" | "ctx_peek" => ToolGuidance {
            name: name.to_string(),
            summary: "Re-cut a buffered large tool response without reloading the full payload."
                .to_string(),
            use_cases: vec![
                "Search within a buffered response for a symbol, path, or error string.".to_string(),
                "Fetch only the first lines or a character slice needed for the next step.".to_string(),
            ],
            avoid_when: vec![
                "Avoid when no response_id is available; call the source tool again if the buffer expired."
                    .to_string(),
            ],
            preconditions: vec![
                "Use response_id from a buffered tool response or omit it to target the latest capture."
                    .to_string(),
            ],
            follow_ups: vec!["ctx_stats".to_string()],
            example: serde_json::json!({"pattern": "AuthClient", "before": 2, "after": 2}),
        },
        _ => fallback(),
    }
}
