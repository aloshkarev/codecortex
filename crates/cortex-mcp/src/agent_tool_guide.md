# CodeCortex MCP — full tool routing guide

Use this as the **appendix** after reading server `instructions` from `initialize`. Fetch this resource at `codecortex://guide/tool-routing` when you need deeper routing detail without rereading every tool description.

## Prerequisites

1. **Graph backend** (Memgraph/Neo4j-compatible) must be reachable per user config (`memgraph_uri`).
2. **Indexed code**: Most graph tools assume `add_code_to_graph` was run on the repo (or equivalent indexing). If results are empty, index first, then retry.
3. **Vector search**: `vector_search`, `vector_search_hybrid`, `search_across_projects`, `vector_index_status`, and vector portions of `get_context_capsule` need vector store + embedder configured. If unavailable, use `find_code`, graph navigation, and `analyze_code_relationships` instead.
4. **Current project**: `go_to_definition`, `find_all_usages`, `quick_info`, `branch_structural_diff`, `pr_review`, and related navigation tools expect `set_current_project` (or a project-aware client). If you get “no current project”, set it before navigation.

## Intent → tool (quick map)

| User intent | Prefer these tools |
|-------------|-------------------|
| Index or re-index | `add_code_to_graph` (optional `include_vector`); `watch_directory` for auto reindex |
| “Find symbol / pattern / name” | `find_code` (kind: name/pattern/type/content) |
| Natural language “where is X done” | `vector_search` or `vector_search_hybrid` after indexing; fallback `find_code` + `analyze_code_relationships` |
| Callers / callees / chain / hierarchy / importers | `analyze_code_relationships` with appropriate `query_type` |
| Dead code | `find_dead_code` or `analyze_code_relationships` + `query_type=dead_code` |
| Go to def / usages / hover-like info | `go_to_definition`, `find_all_usages`, `quick_info` |
| Complexity | `calculate_cyclomatic_complexity` |
| Impact / blast radius | `get_impact_graph` (summary); pair with `analyze_code_relationships` for detail |
| Path between functions | `search_logic_flow` |
| Refactor prep | `analyze_refactoring` |
| Tests for symbol | `find_tests` |
| Cross-repo similarity / shared deps / API diff | `find_similar_across_projects`, `find_shared_dependencies`, `compare_api_surface` |
| Branch / PR review | `branch_structural_diff`, `pr_review` |
| Ad-hoc graph reporting | `execute_cypher_query` only when higher-level tools are insufficient |
| Bundles | `export_bundle`, `load_bundle` |
| Health | `check_health`, `diagnose`, `index_status` |
| Projects & branches | `list_projects`, `add_project`, `set_current_project`, `list_branches`, `project_sync`, `project_status`, … |
| Memory | `save_observation`, `get_session_context`, `search_memory` |
| LSP augmentation | `submit_lsp_edges` |
| Vector lifecycle | `vector_index_repository`, `vector_index_file`, `vector_index_status`, `vector_delete_repository` |

## Disambiguation

- **`find_code`** — structured search over the **code graph** (fast, exact/pattern/name).
- **`vector_search` / `vector_search_hybrid`** — **semantic** search; needs vector index.
- **`execute_cypher_query`** — full power, but easier to get wrong; use last.
- **`get_context_capsule`** — bounded, task-oriented bundle of snippets (good for “give me context for task X”).
- **`get_skeleton`** — file outline from disk; does not require graph (but graph tools often need index elsewhere).

## Destructive / write tools (confirm with user when appropriate)

`delete_repository`, `vector_delete_repository`, `remove_project`, `workspace_setup` (writes files), `export_bundle` / `submit_lsp_edges` / `save_observation` / indexing tools mutate external state or graph.

---

_End of guide._
