//! Static MCP guides, resources, prompts, and agent tool-routing helpers.

use crate::contracts::FreshnessState;
use crate::{ToolCard, output_token_hint, tool_cards, tool_guidance_for, tool_metadata_for};
use rmcp::model::{Annotated, Prompt, PromptArgument, RawResource, Resource};
use serde_json::{Value, json};

pub(crate) fn infer_agent_intent(
    task: &str,
    explicit: Option<&str>,
    artifact: Option<&str>,
    symbol_hint: Option<&str>,
) -> String {
    if let Some(intent) = explicit {
        let normalized = intent.trim().to_ascii_lowercase();
        if !normalized.is_empty() {
            return normalized;
        }
    }

    if let Some(art) = artifact {
        let a = art.trim().to_ascii_lowercase();
        if !a.is_empty() {
            match a.as_str() {
                "navigate" | "navigation" | "goto" => return "navigate".to_string(),
                "bugfix" | "fix" | "patch" => return "patch".to_string(),
                "review" | "pr" => return "review".to_string(),
                "explore" | "discovery" => return "search".to_string(),
                "incident" | "outage" | "triage" => return "incident".to_string(),
                _ => {}
            }
        }
    }

    if symbol_hint.is_some_and(|s| !s.trim().is_empty()) && task.trim().is_empty() {
        return "navigate".to_string();
    }

    let task = task.to_ascii_lowercase();
    if any_contains(
        &task,
        &["review", "pr", "pull request", "diff", "branch", "merge"],
    ) {
        "review".to_string()
    } else if any_contains(
        &task,
        &["stale", "freshness", "repair index", "reindex", "diagnose"],
    ) {
        // "diagnose" overlaps with incident; keep freshness first for index repair wording
        "freshness".to_string()
    } else if any_contains(
        &task,
        &[
            "incident",
            "outage",
            "on-call",
            "on call",
            "sev0",
            "sev 0",
            "sev-0",
            "postmortem",
        ],
    ) {
        "incident".to_string()
    } else if any_contains(
        &task,
        &[
            "go to",
            "goto",
            "definition of",
            "where is",
            "navigate to",
            "jump to",
        ],
    ) || (symbol_hint.is_some_and(|s| !s.trim().is_empty())
        && any_contains(
            &task,
            &["definition", "symbol", "usage", "references", "usages"],
        ))
    {
        "navigate".to_string()
    } else if any_contains(&task, &["test", "spec", "coverage", "flaky"]) {
        "test".to_string()
    } else if any_contains(
        &task,
        &[
            "impact",
            "caller",
            "callers",
            "callee",
            "callees",
            "who calls",
            "what calls",
            "blast",
            "flow",
            "dependency",
            "relationship",
        ],
    ) {
        "impact".to_string()
    } else if any_contains(
        &task,
        &[
            "find",
            "where",
            "search",
            "discover",
            "locate",
            "understand",
        ],
    ) {
        "search".to_string()
    } else if any_contains(
        &task,
        &["project", "branch status", "current repo", "workspace"],
    ) {
        "project".to_string()
    } else {
        "patch".to_string()
    }
}

fn any_contains(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub(crate) fn recommended_tool_sequence(
    intent: &str,
    freshness: &str,
    graph_only: bool,
) -> Vec<&'static str> {
    let mut tools = if matches!(freshness, "stale" | "partial" | "unknown") {
        vec!["check_health", "index_status"]
    } else {
        vec!["check_health"]
    };

    let intent_tools: Vec<&'static str> = match intent {
        "navigate" => vec![
            "go_to_definition",
            "get_signature",
            "quick_info",
            "get_api_contract",
            "find_all_usages",
        ],
        "incident" => vec![
            "explain_index_freshness",
            "get_impact_graph",
            "get_test_context",
            "diagnose",
            "index_status",
        ],
        "review" => vec![
            "get_delta_context",
            "branch_structural_diff",
            "get_impact_graph",
            "get_test_context",
        ],
        "freshness" => vec!["index_status", "explain_index_freshness", "project_sync"],
        "test" => vec!["get_test_context", "find_tests", "get_patch_context"],
        "impact" => vec![
            "get_impact_graph",
            "analyze_code_relationships",
            "search_logic_flow",
            "get_test_context",
        ],
        "search" => {
            if graph_only {
                vec![
                    "find_code",
                    "get_api_contract",
                    "get_signature",
                    "get_context_capsule",
                ]
            } else {
                vec![
                    "find_code",
                    "vector_search_hybrid",
                    "get_context_capsule",
                    "get_api_contract",
                ]
            }
        }
        "project" => vec!["get_current_project", "project_status", "list_branches"],
        _ => {
            if graph_only {
                vec![
                    "estimate_context_cost",
                    "find_code",
                    "get_patch_context",
                    "get_api_contract",
                    "get_test_context",
                    "get_skeleton",
                ]
            } else {
                vec![
                    "estimate_context_cost",
                    "get_patch_context",
                    "get_api_contract",
                    "get_test_context",
                    "get_skeleton",
                ]
            }
        }
    };

    for tool in intent_tools {
        if !tools.contains(&tool) {
            tools.push(tool);
        }
    }
    tools
}

pub(crate) fn metadata_safe_fallbacks(intent: &str, graph_only: bool) -> Vec<&'static str> {
    match intent {
        "review" => vec![
            "branch_structural_diff",
            "get_impact_graph",
            "get_test_context",
        ],
        "search" => {
            let mut v = vec!["find_code", "get_api_contract", "get_signature"];
            if graph_only {
                v.push("get_context_capsule");
            } else {
                v.push("vector_search_hybrid");
            }
            v
        }
        "navigate" => vec!["go_to_definition", "get_signature", "quick_info"],
        "incident" => vec![
            "index_status",
            "explain_index_freshness",
            "get_impact_graph",
        ],
        "patch" => vec!["get_api_contract", "get_test_context", "get_skeleton"],
        _ => vec!["get_tool_guidance", "explain_result"],
    }
}

pub(crate) fn recommendation_entry(
    name: &str,
    priority: usize,
    intent: &str,
    allow_source: bool,
    task: Option<&str>,
    budget_tokens: Option<usize>,
) -> Option<Value> {
    let card = tool_card_for(name)?;
    let hint = output_token_hint(&card.metadata);
    Some(json!({
        "priority": priority,
        "tool": name,
        "reason": recommendation_reason(name, intent),
        "arguments": recommendation_arguments(name, intent, task, budget_tokens),
        "requires_source_permission": card.metadata.can_return_source && !allow_source,
        "estimated_output_tokens_hint": hint,
        "metadata": {
            "cost_class": card.metadata.cost_class,
            "timeout_tier": card.metadata.timeout_tier,
            "minimum_index_tier": card.metadata.minimum_index_tier,
            "token_policy": card.metadata.token_policy,
            "privacy_risk": card.metadata.privacy_risk,
            "can_return_source": card.metadata.can_return_source
        },
        "guidance": card.guidance
    }))
}

pub(crate) fn recommendation_arguments(
    name: &str,
    intent: &str,
    task: Option<&str>,
    budget_tokens: Option<usize>,
) -> Value {
    let task_text = task
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("describe the user task here");
    let budget = budget_tokens.unwrap_or(6000);
    match name {
        "check_health" | "index_status" | "explain_index_freshness" | "diagnose" => json!({}),
        "recommend_tools" => json!({
            "task": task_text,
            "intent": intent,
            "budget_tokens": budget
        }),
        "get_patch_context" => json!({
            "task": task_text,
            "budget_tokens": budget
        }),
        "get_delta_context" => json!({
            "budget_tokens": budget
        }),
        "get_test_context" => json!({
            "symbol": "TargetSymbol",
            "budget_tokens": budget.min(8000)
        }),
        "get_api_contract" | "get_signature" => json!({
            "symbol": "TargetSymbol"
        }),
        "get_context_capsule" => json!({
            "query": task_text,
            "max_tokens": budget
        }),
        "get_impact_graph" => json!({
            "symbol": "TargetSymbol",
            "depth": 2
        }),
        "find_code" => json!({
            "query": task_text,
            "kind": "pattern"
        }),
        "go_to_definition" | "find_all_usages" => json!({
            "symbol": "TargetSymbol"
        }),
        "get_skeleton" => json!({
            "path": "src/module.rs"
        }),
        "pr_review" => json!({
            "budget_tokens": budget
        }),
        "vector_search" | "vector_search_hybrid" => json!({
            "query": task_text,
            "limit": 10
        }),
        "manage_codecortex" => json!({
            "action": "status"
        }),
        _ => json!({}),
    }
}

fn recommendation_reason(name: &str, intent: &str) -> String {
    match name {
        "check_health" => {
            "Cheap preflight to catch unavailable graph/analyzer services.".to_string()
        }
        "index_status" => "Freshness gate before trusting graph/vector-backed context.".to_string(),
        "estimate_context_cost" => "Avoid oversized context requests before retrieval.".to_string(),
        "get_patch_context" => {
            "Structured edit context with contracts, likely tests, and risks.".to_string()
        }
        "get_delta_context" => "Token-bounded branch/change context for review.".to_string(),
        "get_api_contract" => "Signature-level context with lower source exposure.".to_string(),
        "get_test_context" => {
            "Focused validation targets for the changed symbol or task.".to_string()
        }
        "vector_search_hybrid" => "Semantic discovery when symbol names are unknown.".to_string(),
        _ => format!("Relevant to the inferred `{intent}` workflow."),
    }
}

pub(crate) fn recommendation_warnings(
    freshness: &str,
    allow_source: bool,
    budget_tokens: Option<usize>,
    graph_only: bool,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if matches!(freshness, "stale" | "partial" | "unknown") {
        warnings.push(
            "Freshness is not proven fresh; repair or qualify impact-sensitive conclusions."
                .to_string(),
        );
    }
    if !allow_source {
        warnings.push(
            "Source-returning tools were filtered; use metadata/signature tools first.".to_string(),
        );
    }
    if graph_only {
        warnings.push(
            "graph_only routing: vector-heavy tools are deferred; enable graph_only=false and vector read to use hybrid semantic search."
                .to_string(),
        );
    }
    if budget_tokens.is_some_and(|budget| budget < 3000) {
        warnings.push("Low token budget; prefer signatures, skeletons, and summaries.".to_string());
    }
    warnings
}

pub(crate) fn freshness_state_from_label(label: &str) -> FreshnessState {
    match label {
        "fresh" => FreshnessState::Fresh,
        "warming" => FreshnessState::Warming,
        "stale" => FreshnessState::Stale,
        "partial" => FreshnessState::Partial,
        _ => FreshnessState::Unknown,
    }
}

pub(crate) fn tool_card_for(name: &str) -> Option<ToolCard> {
    tool_metadata_for(name).copied().map(|metadata| ToolCard {
        metadata,
        guidance: tool_guidance_for(name),
    })
}

pub(crate) fn codecortex_server_instructions() -> &'static str {
    "CodeCortex is a local-first semantic layer between AI coding agents and repository state: it exposes discoverable docs and tool contracts (MCP resources and catalog), bounded operations (MCP tools), and inspectable health/freshness/diagnostics (structured fields on responses).\n\
Start critical workflows with manage_codecortex or check_health and index_status. Prefer get_patch_context before editing, get_api_contract for signatures, get_test_context before changing tests, and get_delta_context for branch review. Use scoped filters and token budgets. Treat stale, partial, or unknown freshness as a warning: avoid high-confidence impact claims until the index is repaired. Source snippets are bounded and redacted; prefer metadata, signatures, skeletons, and context packs over broad full-file reads. Use resources/list and prompts/list for tool-routing guides and reusable workflows. Read codecortex://guide/mcp-protocol for transports, capabilities, and protocol gaps (roots/sampling). When the user also runs a backend-for-agents stack (e.g. InsForge), read codecortex://guide/agent-platforms to split repo intelligence vs backend operations and avoid the wrong tool for each question."
}

pub(crate) fn codecortex_resources() -> Vec<Resource> {
    vec![
        resource(
            "codecortex://guide/tool-routing",
            "tool-routing",
            "How agents should choose CodeCortex MCP tools by task type, cost, freshness, and privacy.",
            "text/markdown",
        ),
        resource(
            "codecortex://guide/agent-workflows",
            "agent-workflows",
            "Recommended preflight, patch, review, test, and diagnostics workflows for AI agents.",
            "text/markdown",
        ),
        resource(
            "codecortex://guide/privacy",
            "privacy",
            "Local-first privacy, source exposure, redaction, and remote MCP/embedding guidance.",
            "text/markdown",
        ),
        resource(
            "codecortex://guide/mcp-protocol",
            "mcp-protocol",
            "MCP/rmcp surface, transports, declared capabilities, and intentional protocol gaps (roots, sampling).",
            "text/markdown",
        ),
        resource(
            "codecortex://guide/agent-platforms",
            "agent-platforms",
            "How CodeCortex pairs with backend semantic layers (e.g. InsForge): roles, routing, discover/act/verify, multi-MCP setups.",
            "text/markdown",
        ),
        resource(
            "codecortex://guide/agent-pack-bootstrap",
            "agent-pack-bootstrap",
            "Install skills, subagents, hooks, rules, and project MCP config via workspace_setup or manage_codecortex.",
            "text/markdown",
        ),
        resource(
            "codecortex://guide/a2a",
            "a2a",
            "Hybrid MCP + A2A orchestration, config.toml settings, HTTP endpoints, and blackboard.",
            "text/markdown",
        ),
        resource(
            "codecortex://tools/catalog",
            "tools-catalog",
            "JSON catalog of MCP tools with metadata, guidance, examples, and follow-up tools.",
            "application/json",
        ),
        resource(
            "codecortex://schema/context-pack",
            "context-pack-schema",
            "JSON schema-style contract for bounded context pack responses.",
            "application/schema+json",
        ),
    ]
}

fn resource(uri: &str, name: &str, description: &str, mime_type: &str) -> Resource {
    Annotated::new(
        RawResource::new(uri, name)
            .with_description(description)
            .with_mime_type(mime_type),
        None,
    )
}

pub(crate) fn codecortex_resource_text(uri: &str) -> Option<(&'static str, String)> {
    match uri {
        "codecortex://guide/tool-routing" => Some(("text/markdown", tool_routing_guide())),
        "codecortex://guide/agent-workflows" => Some(("text/markdown", agent_workflows_guide())),
        "codecortex://guide/privacy" => Some(("text/markdown", privacy_guide())),
        "codecortex://guide/mcp-protocol" => {
            Some(("text/markdown", crate::mcp_protocol::mcp_protocol_guide()))
        }
        "codecortex://guide/agent-platforms" => Some(("text/markdown", agent_platforms_guide())),
        "codecortex://guide/agent-pack-bootstrap" => {
            Some(("text/markdown", agent_pack_bootstrap_guide()))
        }
        "codecortex://guide/a2a" => Some(("text/markdown", a2a_guide())),
        "codecortex://tools/catalog" => Some((
            "application/json",
            serde_json::to_string_pretty(&tool_cards()).unwrap_or_else(|_| "[]".to_string()),
        )),
        "codecortex://schema/context-pack" => Some((
            "application/schema+json",
            serde_json::to_string_pretty(&context_pack_schema())
                .unwrap_or_else(|_| "{}".to_string()),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod static_resource_tests {
    use super::codecortex_resource_text;

    #[test]
    fn mcp_protocol_guide_resolves() {
        assert!(codecortex_resource_text("codecortex://guide/mcp-protocol").is_some());
    }

    #[test]
    fn agent_platforms_guide_resolves() {
        assert!(codecortex_resource_text("codecortex://guide/agent-platforms").is_some());
    }

    #[test]
    fn context_pack_schema_has_recommend_tools_contract() {
        let schema = super::context_pack_schema();
        assert!(
            schema.get("recommend_tools_response").is_some(),
            "schema should document recommend_tools orchestration contract"
        );
    }

    #[test]
    fn recommendation_entry_includes_arguments_field() {
        let entry = super::recommendation_entry(
            "get_patch_context",
            1,
            "patch",
            false,
            Some("fix auth refresh"),
            Some(6000),
        )
        .expect("entry");
        let args = entry
            .get("arguments")
            .expect("arguments field")
            .as_object()
            .expect("arguments object");
        assert_eq!(
            args.get("task").and_then(|v| v.as_str()),
            Some("fix auth refresh")
        );
        assert_eq!(
            args.get("budget_tokens").and_then(|v| v.as_u64()),
            Some(6000)
        );
    }
}

pub(crate) fn codecortex_prompts() -> Vec<Prompt> {
    vec![
        prompt(
            "codecortex_patch_plan",
            "Plan a safe code edit using CodeCortex context tools before reading broad source.",
            vec![
                ("task", "The user-requested change or bugfix.", true),
                (
                    "scope",
                    "Optional path/module scope such as src/auth.",
                    false,
                ),
                ("budget_tokens", "Optional context budget.", false),
            ],
        ),
        prompt(
            "codecortex_branch_review",
            "Review a branch using delta context, impact graph, likely tests, and privacy-safe evidence.",
            vec![
                ("source_branch", "Branch to review.", true),
                ("target_branch", "Base branch, usually main.", false),
            ],
        ),
        prompt(
            "codecortex_freshness_repair",
            "Diagnose stale/partial/unknown index state and propose exact repair commands.",
            vec![("repo_path", "Repository path to inspect.", false)],
        ),
        prompt(
            "codecortex_incident_triage",
            "Triage an incident with health, index freshness, impact graph, and tests under tight evidence rules.",
            vec![
                ("symptom", "User-visible failure or alert text.", true),
                ("repo_path", "Repository path in scope.", false),
            ],
        ),
        prompt(
            "codecortex_a2a_consensus",
            "Run A2A consensus_review: spawn session, poll task, interpret Reject/FinalResult with freshness guardrails.",
            vec![
                (
                    "task",
                    "Change description for the multi-agent session.",
                    true,
                ),
                ("include_paths", "Optional path scope.", false),
            ],
        ),
    ]
}

fn prompt(name: &str, description: &str, args: Vec<(&str, &str, bool)>) -> Prompt {
    Prompt::new(
        name,
        Some(description),
        Some(
            args.into_iter()
                .map(|(name, description, required)| {
                    PromptArgument::new(name)
                        .with_description(description)
                        .with_required(required)
                })
                .collect(),
        ),
    )
}

pub(crate) fn codecortex_prompt_text(name: &str) -> Option<(&'static str, String)> {
    match name {
        "codecortex_patch_plan" => Some((
            "Plan an edit with bounded CodeCortex context.",
            "Use CodeCortex as follows:\n\
1. Call check_health.\n\
2. Call index_status with the repository path. If freshness is stale, partial, or unknown, explain the risk and repair first when the task depends on impact accuracy.\n\
3. Call estimate_context_cost for the task and scope.\n\
4. Call get_patch_context with task, include_paths when known, mode, and budget_tokens.\n\
5. Follow next_tools from the response, usually get_api_contract, get_test_context, and get_skeleton.\n\
6. Read only the exact files/spans still needed. Avoid broad source reads.\n\
7. After editing, run focused tests from get_test_context and use get_delta_context for a short review.".to_string(),
        )),
        "codecortex_branch_review" => Some((
            "Review a branch with structural and token-bounded context.",
            "Use CodeCortex as follows:\n\
1. Call check_health and index_status.\n\
2. Call get_delta_context with source_branch and target_branch.\n\
3. Use branch_structural_diff for deeper structural changes.\n\
4. Use get_impact_graph for high-risk changed symbols.\n\
5. Use get_test_context for likely validation.\n\
6. Report findings with freshness, paths/symbols, risks, and missing tests. Do not claim complete safety if freshness is stale/partial/unknown.".to_string(),
        )),
        "codecortex_freshness_repair" => Some((
            "Repair stale CodeCortex graph/vector state.",
            "Use CodeCortex as follows:\n\
1. Call index_status with include_jobs and include_watcher.\n\
2. Call explain_index_freshness for repair commands.\n\
3. If graph is stale, run or suggest incremental/full indexing.\n\
4. If vector is stale or unavailable, use vector_index_status and vector indexing repair.\n\
5. If watcher is not running, start watch_directory or cortex watch.\n\
6. Re-run index_status and only then proceed with impact-sensitive tools.".to_string(),
        )),
        "codecortex_incident_triage" => Some((
            "Bounded incident triage with CodeCortex.",
            "Use CodeCortex as follows:\n\
1. Call check_health and index_status for the affected repo.\n\
2. Call explain_index_freshness if freshness is stale, partial, or unknown before blast-radius claims.\n\
3. Call get_impact_graph on the suspected symbol or entrypoint.\n\
4. Call get_test_context for likely regression tests.\n\
5. Call diagnose if local tooling reports errors.\n\
6. Summarize with explicit freshness caveats; do not claim root cause if the graph is not fresh.".to_string(),
        )),
        "codecortex_a2a_consensus" => Some((
            "A2A consensus review with CodeCortex graph-backed roles.",
            "Use CodeCortex as follows:\n\
1. Call check_health and index_status; repair if freshness is stale, partial, or unknown.\n\
2. Call cortex_a2a_spawn_session with workflow consensus_review, include_paths, and return_immediately true.\n\
3. Poll cortex_a2a_get_task with spec_json true until status is terminal.\n\
4. Interpret Reject events as required revisions; FinalResult when completed.\n\
5. Use get_patch_context / get_impact_graph directly only when you need detail beyond the session blackboard.".to_string(),
        )),
        _ => None,
    }
}

fn tool_routing_guide() -> String {
    let mut out = String::from(
        "# CodeCortex Tool Routing\n\n\
Always start with `check_health` and `index_status` for critical work. Use path filters, token budgets, and the cheapest tool that can answer the question.\n\n\
| Task | First Tool | Follow-ups |\n\
| --- | --- | --- |\n\
| Choose an efficient workflow | `recommend_tools` | `get_tool_guidance` |\n\
| Pre-edit planning | `get_patch_context` | `get_api_contract`, `get_test_context`, `get_skeleton` |\n\
| General context | `get_context_capsule` | `get_signature`, `get_skeleton` |\n\
| Branch review | `get_delta_context` | `branch_structural_diff`, `get_impact_graph` |\n\
| Callers/callees/blast radius | `get_impact_graph` | `analyze_code_relationships`, `get_test_context` |\n\
| Natural-language discovery | `vector_search_hybrid` | `get_context_capsule`, `get_api_contract` |\n\
| Raw graph query | `execute_cypher_query` | Only when typed tools cannot answer |\n\
| Re-cut buffered response | `ctx_stats` | `ctx_peek`, `ctx_grep`, `ctx_slice` |\n\n\
## Tool Cards\n\n",
    );
    for card in tool_cards() {
        out.push_str(&format!(
            "### `{}`\n- Cost: `{:?}`\n- Index: `{:?}`\n- Privacy: `{:?}`\n- Summary: {}\n- Use cases: {}\n- Follow-ups: {}\n\n",
            card.metadata.name,
            card.metadata.cost_class,
            card.metadata.minimum_index_tier,
            card.metadata.privacy_risk,
            card.guidance.summary,
            card.guidance.use_cases.join("; "),
            card.guidance.follow_ups.join(", "),
        ));
    }
    out
}

fn agent_workflows_guide() -> String {
    "# CodeCortex Agent Workflows\n\n\
## Semantic layer loop\n\
Aligns with common agent-backend patterns (e.g. discover capabilities, act through a narrow API, verify state):\n\n\
1. **Discover** — `resources/list` / `resources/read` for `codecortex://guide/tool-routing`, `codecortex://guide/agent-workflows`, and `codecortex://tools/catalog` (or call `recommend_tools` first).\n\
2. **Act** — call scoped MCP tools with filters and token budgets; prefer `get_patch_context`, `get_delta_context`, and graph helpers over ad-hoc full-file reads.\n\
3. **Verify** — `check_health`, `index_status`, `diagnose`, and response `freshness` / `source_policy` before high-confidence impact or safety claims.\n\n\
## Full-stack agents (code + backend)\n\
When the user also connects a **backend-for-agents** MCP (e.g. [InsForge](https://github.com/InsForge/InsForge)):\n\
- Use **CodeCortex** for repository truth: call graphs, tests around a change, patch/delta context, cyclomatic hotspots.\n\
- Use the **backend platform** for operational truth: schema migrations, auth configuration, buckets, gateway models, deployment—not for symbol-level repo analysis.\n\
- Run the same **discover → act → verify** loop on both sides; never substitute one for the other.\n\
- Read `codecortex://guide/agent-platforms` when both stacks are in play.\n\
## Tool Selection\n\
Call `recommend_tools` with the user task, known scope, token budget, source permission, and freshness when available. Use `get_tool_guidance` for a specific tool before calling an unfamiliar or expensive tool.\n\n\
## Patch Planning\n\
1. `check_health`\n\
2. `index_status`\n\
3. `estimate_context_cost`\n\
4. `get_patch_context`\n\
5. `get_api_contract` for target symbols\n\
6. `get_test_context`\n\
7. targeted file reads and edit\n\n\
## Branch Review\n\
1. `index_status`\n\
2. `get_delta_context`\n\
3. `branch_structural_diff`\n\
4. `get_impact_graph` on risky symbols\n\
5. `get_test_context`\n\n\
## Diagnostics\n\
Use `diagnose`, `explain_index_freshness`, `project_status`, and `project_sync` before trusting stale results.\n"
        .to_string()
}

fn agent_pack_bootstrap_guide() -> String {
    r#"# CodeCortex agent pack bootstrap

Install Cursor skills, subagents, advisory hooks, rules, and project MCP config from the packaged agent pack.

## Resolution order

1. `CORTEX_AGENT_PACK` environment variable (absolute path to `plugin/codecortex` layout)
2. Walk parents from the repo for `plugin/codecortex`
3. `../share/codecortex-agent-pack` next to the `cortex` binary

## MCP tools

| Tool | When |
| --- | --- |
| `manage_codecortex` | Session start: `action=assess`, optional `enable_watch=true` |
| `manage_codecortex` | First-time repo: `action=bootstrap`, `install_agent_pack=true` |
| `workspace_setup` | Explicit install: `install_agent_pack=true`, `generate_configs=true` |

## MCP server id

Generated configs use **`codecortex`** pointing at `cortex mcp start` with `cwd` set to the repo root. Enable globally in `~/.cursor/mcp.json` per docs/INTEGRATION.md if needed.

## Shell fallback

```bash
./plugin/codecortex/cursor/install.sh
export CORTEX_AGENT_PACK=/path/to/plugin/codecortex
```

## Watch

Pass `enable_watch=true` to start `watch_directory` for the repo (or `watch_paths`). Repairs remain opt-in via `auto_repair=true` on `manage_codecortex`.
"#
    .to_string()
}

fn a2a_guide() -> String {
    include_str!("../../../docs/A2A.md").to_string()
}

fn agent_platforms_guide() -> String {
    r#"# CodeCortex alongside backend agent platforms

[InsForge](https://github.com/InsForge/InsForge) is an example of a **backend-for-agents** stack (DB, auth, storage, model gateway, deploy). CodeCortex does not replace it; pair both MCP servers and route by question type.

## What to borrow from InsForge-style design

| Idea | InsForge-style behavior | CodeCortex equivalent |
| --- | --- | --- |
| Discover | Agents pull docs/operations via MCP (e.g. `fetch-docs`) | `resources/read` on `codecortex://guide/tool-routing`, `codecortex://guide/agent-workflows`, and `codecortex://tools/catalog` |
| Act | Configure and invoke narrow backend APIs | Scoped MCP tools + token budgets + `source_policy` |
| Inspect | Structured logs/state for trust | `check_health`, `index_status`, `freshness`, `diagnose`, context-pack `meta` |
| Onboarding | Short verification prompt after connect | Run **Verification** below after MCP enable |
| Multi-project | Separate ports/instances per project | Separate MCP instances or repo roots per codebase; do not assume one index covers unrelated trees |

## Routing: which MCP answers which question?

| User intent | Prefer |
| --- | --- |
| Who calls `X`, blast radius, dead code, complexity | CodeCortex |
| Patch plan, API contract, tests to run for a change | CodeCortex |
| Branch diff + structural impact | CodeCortex |
| DB schema, RLS policies, migrations | Backend platform / DB MCP |
| Auth provider config, sessions, OAuth | Backend platform |
| Object storage buckets, signed URLs | Backend platform |
| Which LLM route or gateway key policy | Backend platform |
| Production logs or runtime errors | Backend/observability—not inferred from static graph |

**Rule:** If the answer depends on **live** or **deployed** state, use the backend stack. If it depends on **source structure** or **static** test/relationship data, use CodeCortex (with freshness checks).

## Verification (after connecting CodeCortex MCP)

1. `resources/read` → `codecortex://guide/tool-routing`
2. `check_health`
3. `index_status` on the active repository
4. If `freshness` is `stale`, `partial`, or `unknown`, follow `codecortex://guide/agent-workflows` repair guidance before impact claims.

Optional: ask the user to confirm the backend MCP is connected when the task touches both repo and deployed services.

## Skills and repo layout

InsForge ships agent skills under `.agents/`, `.claude/`, `.codex/`. Mirror that idea for **this** repo: keep `docs/skills/codecortex/SKILL.md` and `AGENTS.md` as the canonical agent entry points; point them at `codecortex://guide/tool-routing` and `codecortex://guide/agent-workflows`.

## Model gateway note

A backend **model gateway** (InsForge-style) centralizes provider keys and routing; it does **not** replace CodeCortex. Continue using CodeCortex for code context; use the gateway only for LLM inference configuration when building or operating the app—not for repository indexing.
"#
    .to_string()
}

fn privacy_guide() -> String {
    "# CodeCortex Privacy Guidance\n\n\
- CodeCortex is local-first; stdio and loopback MCP are safest defaults.\n\
- Remote MCP requires explicit `--allow-remote` and bearer token configuration.\n\
- Prefer signatures, skeletons, metadata, and bounded snippets over full source.\n\
- Source snippets are redacted for common secret-bearing lines before returning.\n\
- Remote embeddings can expose source to the embedding provider; use local embeddings for private repositories.\n\
- Treat each response `source_policy`, `privacy_warnings`, and `freshness` field as part of the evidence contract.\n"
        .to_string()
}

fn context_pack_schema() -> Value {
    json!({
        "type": "object",
        "required": ["status", "meta", "warnings", "data"],
        "properties": {
            "status": {"enum": ["ok", "partial", "error"]},
            "meta": {
                "type": "object",
                "properties": {
                    "freshness": {"enum": ["fresh", "warming", "stale", "partial", "unknown"]},
                    "token_budget": {
                        "type": "object",
                        "properties": {
                            "requested_tokens": {"type": "integer"},
                            "estimated_tokens": {"type": "integer"},
                            "hard_cap": {"type": "boolean"}
                        }
                    },
                    "source_policy": {"enum": ["metadata_only", "signatures", "snippets", "full_source", "forbidden"]},
                    "cost_class": {"enum": ["cheap", "bounded", "expensive", "background"]},
                    "omitted": {"type": "array"},
                    "next_tools": {"type": "array", "items": {"type": "string"}},
                    "privacy_warnings": {"type": "array", "items": {"type": "string"}}
                }
            }
        },
        "recommend_tools_response": {
            "description": "Machine-readable shape of recommend_tools `data` payload for orchestrators",
            "type": "object",
            "properties": {
                "intent": {"type": "string"},
                "mcp_profile": {"enum": ["dev", "strict"]},
                "graph_only": {"type": "boolean"},
                "allow_source": {"type": "boolean"},
                "recommendations": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["tool", "priority"],
                        "properties": {
                            "tool": {"type": "string"},
                            "priority": {"type": "integer"},
                            "reason": {"type": "string"},
                            "requires_source_permission": {"type": "boolean"},
                            "estimated_output_tokens_hint": {"type": "integer"},
                            "metadata": {"type": "object"},
                            "guidance": {"type": "object"}
                        }
                    }
                }
            }
        }
    })
}
