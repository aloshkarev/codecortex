//! Single source for exported tool names and agent-facing routing documentation.
//!
//! `ToolHints` align with MCP `ToolAnnotations` (`read_only_hint`, `destructive_hint`, etc.).

/// URI for the markdown playbook returned by `resources/read`.
pub const TOOL_ROUTING_RESOURCE_URI: &str = "codecortex://guide/tool-routing";

/// Every MCP tool exposed by [`crate::handler::CortexHandler`], alphabetically sorted.
pub const TOOL_NAMES: &[&str] = &[
    "add_code_to_graph",
    "add_project",
    "analyze_code_relationships",
    "analyze_refactoring",
    "branch_structural_diff",
    "calculate_cyclomatic_complexity",
    "check_health",
    "check_job_status",
    "compare_api_surface",
    "delete_repository",
    "diagnose",
    "execute_cypher_query",
    "explain_result",
    "export_bundle",
    "find_all_usages",
    "find_code",
    "find_dead_code",
    "find_patterns",
    "find_shared_dependencies",
    "find_similar_across_projects",
    "find_tests",
    "get_context_capsule",
    "get_current_project",
    "get_impact_graph",
    "get_repository_stats",
    "get_session_context",
    "get_signature",
    "get_skeleton",
    "go_to_definition",
    "index_status",
    "list_branches",
    "list_indexed_repositories",
    "list_jobs",
    "list_projects",
    "list_watched_paths",
    "load_bundle",
    "pr_review",
    "project_branch_diff",
    "project_metrics",
    "project_queue_status",
    "project_status",
    "project_sync",
    "quick_info",
    "refresh_project",
    "remove_project",
    "save_observation",
    "search_across_projects",
    "search_logic_flow",
    "search_memory",
    "set_current_project",
    "submit_lsp_edges",
    "unwatch_directory",
    "vector_delete_repository",
    "vector_index_file",
    "vector_index_repository",
    "vector_index_status",
    "vector_search",
    "vector_search_hybrid",
    "watch_directory",
    "workspace_setup",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolHints {
    pub read_only: bool,
    pub destructive: bool,
    pub idempotent: bool,
    pub open_world: bool,
}

/// Sorted by tool name for binary search.
const HINTS: &[(&str, ToolHints)] = &[
    (
        "add_code_to_graph",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "add_project",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "analyze_code_relationships",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "analyze_refactoring",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "branch_structural_diff",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "calculate_cyclomatic_complexity",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "check_health",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "check_job_status",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "compare_api_surface",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "delete_repository",
        ToolHints {
            read_only: false,
            destructive: true,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "diagnose",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "execute_cypher_query",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "explain_result",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "export_bundle",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "find_all_usages",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "find_code",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "find_dead_code",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "find_patterns",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "find_shared_dependencies",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "find_similar_across_projects",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "find_tests",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "get_context_capsule",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "get_current_project",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "get_impact_graph",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "get_repository_stats",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "get_session_context",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "get_signature",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "get_skeleton",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "go_to_definition",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "index_status",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "list_branches",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "list_indexed_repositories",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "list_jobs",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "list_projects",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "list_watched_paths",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "load_bundle",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "pr_review",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "project_branch_diff",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "project_metrics",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "project_queue_status",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "project_status",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "project_sync",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "quick_info",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "refresh_project",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "remove_project",
        ToolHints {
            read_only: false,
            destructive: true,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "save_observation",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "search_across_projects",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "search_logic_flow",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "search_memory",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "set_current_project",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "submit_lsp_edges",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "unwatch_directory",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "vector_delete_repository",
        ToolHints {
            read_only: false,
            destructive: true,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "vector_index_file",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "vector_index_repository",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "vector_index_status",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "vector_search",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "vector_search_hybrid",
        ToolHints {
            read_only: true,
            destructive: false,
            idempotent: true,
            open_world: false,
        },
    ),
    (
        "watch_directory",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
    (
        "workspace_setup",
        ToolHints {
            read_only: false,
            destructive: false,
            idempotent: false,
            open_world: false,
        },
    ),
];

#[must_use]
pub fn hints_for(tool_name: &str) -> Option<ToolHints> {
    HINTS
        .binary_search_by(|(name, _)| (*name).cmp(tool_name))
        .ok()
        .map(|idx| HINTS[idx].1)
}

/// Concise playbook sent in `initialize.instructions` (keep dense; full guide is a Resource).
#[must_use]
pub fn server_instructions_markdown() -> String {
    format!(
        r#"CodeCortex MCP — how to use tools

## Prerequisite
- Ensure the graph DB is up. Before graph-backed analysis, run **add_code_to_graph** on the repo (or rely on existing index). For NL semantic search, index vectors (**vector_index_repository** / **vector_index_file**) first.
- Navigation tools (**go_to_definition**, **find_all_usages**, **quick_info**, **pr_review**, …) need a **current project**: **set_current_project** after **add_project** if unsure.

## Pick a tool by intent
| Intent | Tool |
|--------|------|
| Index / re-index | add_code_to_graph; optional **include_vector** |
| Live reindex on edits | watch_directory |
| Search symbols / pattern | find_code (kind: name/pattern/type/content) |
| Semantic “where is X handled” | vector_search or vector_search_hybrid (after vector index) |
| Callers / callees / chain / hierarchy | analyze_code_relationships (query_type) |
| Dead code | find_dead_code |
| Go to def / usages / hover | go_to_definition, find_all_usages, quick_info |
| Complexity | calculate_cyclomatic_complexity |
| Impact summary | get_impact_graph |
| Path A → B in call graph | search_logic_flow |
| Refactor impact | analyze_refactoring |
| Tests for symbol | find_tests |
| Cross-repo | find_similar_across_projects, find_shared_dependencies, compare_api_surface |
| Branch / PR | branch_structural_diff, pr_review |
| Custom graph | execute_cypher_query (last resort) |
| Health | check_health, diagnose, index_status |
| Multi-project | list_projects, add_project, set_current_project, project_sync, project_status |
| Memory | save_observation, search_memory, get_session_context |
| Vector lifecycle | vector_index_*, vector_search*, vector_delete_repository |

## Disambiguation
- **find_code** = graph search. **vector_search** = semantic (needs index). **execute_cypher_query** = power user only.
- **get_context_capsule** = bounded task context. **get_skeleton** = file outline from disk.

## Full playbook
Read MCP resource URI: `{uri}` (markdown).
"#,
        uri = TOOL_ROUTING_RESOURCE_URI
    )
}

#[must_use]
pub fn resource_tool_guide_markdown() -> &'static str {
    include_str!("agent_tool_guide.md")
}

/// Prompt name: `prompts/get` → routed workflow checklist.
pub const PROMPT_SESSION_BOOTSTRAP: &str = "codecortex_session_bootstrap";

/// Prompt name: map a goal to tools (optional argument `user_goal`).
pub const PROMPT_ROUTE_TOOLS: &str = "codecortex_route_tools";

#[must_use]
pub fn prompt_session_bootstrap_body() -> String {
    r#"You are using CodeCortex MCP. Follow this order when starting work on a codebase:

1. If needed, **check_health** or **diagnose** to verify the graph DB.
2. **list_projects** / **get_current_project** — register and select scope with **add_project** + **set_current_project** if multi-repo.
3. **list_indexed_repositories** or **index_status** — if the repo is missing or stale, **add_code_to_graph** (set **include_vector** if you need semantic search later).
4. For navigation: **go_to_definition**, **find_all_usages**, **quick_info** on symbols within the current project.
5. For search: **find_code** (structured) or **vector_search** / **vector_search_hybrid** (semantic, if indexed).
6. For deeper appendix and intent→tool table, read resource `codecortex://guide/tool-routing`.

Reply with the minimal next tool calls you would make and why."#
        .to_string()
}

#[must_use]
pub fn prompt_route_tools_body(user_goal: &str) -> String {
    format!(
        r#"The user goal is:

{goal}

Using only CodeCortex MCP tools, list:
1. Which tools you will call **in order**
2. Brief **when** for each
3. **Prerequisites** (index, current project, vector index) if any

Consult server instructions and resource `codecortex://guide/tool-routing` for the full routing table."#,
        goal = user_goal
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_names_match_hints_len_and_sorted_unique() {
        assert_eq!(TOOL_NAMES.len(), HINTS.len());
        for (i, name) in TOOL_NAMES.iter().enumerate() {
            assert_eq!(HINTS[i].0, *name, "TOOL_NAMES and HINTS must align");
        }
        for w in TOOL_NAMES.windows(2) {
            assert!(w[0] < w[1], "TOOL_NAMES must be sorted");
        }
        let mut u = std::collections::HashSet::new();
        for n in TOOL_NAMES {
            assert!(u.insert(*n));
        }
    }

    #[test]
    fn hints_lookup_covers_all_names() {
        for name in TOOL_NAMES {
            assert!(hints_for(name).is_some(), "missing hints for {name}");
        }
    }
}
