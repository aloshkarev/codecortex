# CodeCortex rules, skills, and agents index

Maps user intent → Cursor rule → skill → subagent → first MCP tool. Keep this table aligned with [docs/agents/README.md](../agents/README.md).

| User intent | Rule | Skill | Subagent | MCP start |
| --- | --- | --- | --- | --- |
| Default / any code task | codecortex-core | codecortex | — | `check_health` |
| Stale or missing index | codecortex-indexing | codecortex-indexing | codecortex-indexer | `index_status` |
| Who calls / callees / impact | codecortex-analysis | codecortex | codecortex-analyzer | `analyze_code_relationships` |
| Blast radius (symbol) | codecortex-analysis | codecortex | codecortex-analyzer | `get_impact_graph` |
| Dead code / complexity | codecortex-analysis | codecortex | codecortex-analyzer | `find_dead_code` |
| NL code discovery | codecortex-analysis | codecortex | codecortex-analyzer | `vector_search_hybrid` |
| Plan edit before coding | codecortex-workflows | codecortex-workflows | codecortex-patch-planner | `get_patch_context` |
| Branch / PR review | codecortex-workflows | codecortex-workflows | codecortex-pr-reviewer | `get_delta_context` |
| Tool choice unclear | codecortex-tool-routing | codecortex | — | `recommend_tools` |
| Multi-project scope | codecortex-core | codecortex-workflows | — | `list_projects` |
| Preflight / trust | codecortex-core | codecortex | codecortex-indexer | `check_health` |
| Delegate long chains | codecortex-subagents | — | see matrix | — |
| Multi-step A2A orchestration | codecortex-a2a | codecortex-workflows | hub + roles | `cortex_a2a_spawn_session` |

## MCP categories (compact)

| Category | Start tool | Index tier |
| --- | --- | --- |
| Preflight | `check_health` → `index_status` | none |
| Routing | `recommend_tools` | none |
| Indexing | `add_code_to_graph` | writes index |
| Search | `find_code` | graph |
| Relationships | `get_impact_graph` | graph |
| Navigation | `go_to_definition` | graph |
| Quality | `find_dead_code` | graph |
| Vector / hybrid | `vector_search_hybrid` | graph_and_vector |
| Context packs | `get_patch_context` | graph / graph_and_vector |
| Project | `list_projects` | project |
| Jobs / diagnostics | `diagnose` | none |

Full tool metadata: `codecortex://tools/catalog` or `cortex mcp tools --metadata`.

## File locations

| Type | Path |
| --- | --- |
| Rules | `.cursor/rules/codecortex-*.mdc` |
| Hooks | `docs/cursor/hooks.json` (via `.cursor/hooks.json`) |
| Skills | `docs/skills/` → `.cursor/skills/` |
| Agents | `docs/agents/` → `.cursor/agents/` |
