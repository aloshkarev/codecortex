# MCP tool matrix (compact)

Authoritative per-tool metadata lives in `codecortex://tools/catalog`, `get_tool_guidance`, and `cortex mcp tools --metadata`. This table is for quick routing only.

| Category | Start here | Minimum index tier | Notes |
| --- | --- | --- | --- |
| Preflight / trust | `check_health` → `index_status` | `none` | Always before impact claims |
| Routing help | `recommend_tools` | `none` | Cheaper than full catalog |
| Indexing / watch | `add_code_to_graph`, `watch_directory` | `none` (writes index) | Background class; see codecortex-indexing |
| Search / localize | `find_code` | `graph` | Bounded snippets |
| Relationships / impact | `get_impact_graph`, `analyze_code_relationships` | `graph` | Expensive; scope first |
| Navigation | `go_to_definition`, `find_all_usages`, `quick_info` | `graph` | Cheap to bounded |
| Quality / refactor | `find_dead_code`, `calculate_cyclomatic_complexity`, `analyze_refactoring` | `graph` | Filter by path |
| Vector / hybrid | `vector_search_hybrid` | `graph_and_vector` | NL discovery; needs both indexes |
| Agent context | `get_patch_context`, `get_delta_context`, `get_context_capsule` | `graph` or `graph_and_vector` | Token budgets; see codecortex-workflows |
| Project / multi-repo | `list_projects`, `set_current_project`, cross-project tools | `project` / `graph` | Set project before single-repo assumptions |
| Jobs / diagnostics | `list_jobs`, `diagnose`, `explain_index_freshness` | `none` | After background index/sync |

Cost classes: `cheap` < `bounded` < `expensive` < `background`. Prefer the cheapest tool that can answer the question.
