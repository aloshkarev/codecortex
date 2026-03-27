# cortex-mcp

> `cortex-mcp` is the MCP server crate for CodeCortex. It receives JSON-RPC requests and routes them into graph, analyzer, vector, project, and memory operations, exposing **60 tools** to AI clients over stdio, HTTP-SSE, WebSocket, or multi-transport.

## What it does

- Routes all 60 CodeCortex tools through a single `CortexHandler` regardless of transport
- Exposes a prompt (`codecortex_route_tools`) and resource (`codecortex://guide/tool-routing`) for AI agent routing guidance
- Controls tool availability at runtime via environment-variable feature flags
- Supports bearer token authentication for network transports

## Tool catalog (60 tools, alphabetical)

| Tool | Area |
|------|------|
| `add_code_to_graph` | Indexing |
| `add_project` | Project |
| `analyze_code_relationships` | Analysis |
| `analyze_refactoring` | Analysis |
| `branch_structural_diff` | Review |
| `calculate_cyclomatic_complexity` | Analysis |
| `check_health` | Health |
| `check_job_status` | Jobs |
| `compare_api_surface` | Cross-project |
| `delete_repository` | Repository |
| `diagnose` | Health |
| `execute_cypher_query` | Query |
| `explain_result` | Explain |
| `export_bundle` | Bundle |
| `find_all_usages` | Navigation |
| `find_code` | Search |
| `find_dead_code` | Analysis |
| `find_patterns` | Analysis |
| `find_shared_dependencies` | Cross-project |
| `find_similar_across_projects` | Cross-project |
| `find_tests` | Tests |
| `get_context_capsule` | Context |
| `get_current_project` | Project |
| `get_impact_graph` | Context |
| `get_repository_stats` | Repository |
| `get_session_context` | Memory |
| `get_signature` | Context |
| `get_skeleton` | Context |
| `go_to_definition` | Navigation |
| `index_status` | Indexing |
| `list_branches` | Project |
| `list_indexed_repositories` | Repository |
| `list_jobs` | Jobs |
| `list_projects` | Project |
| `list_watched_paths` | Watch |
| `load_bundle` | Bundle |
| `pr_review` | Review |
| `project_branch_diff` | Project |
| `project_metrics` | Project |
| `project_queue_status` | Project |
| `project_status` | Project |
| `project_sync` | Project |
| `quick_info` | Navigation |
| `refresh_project` | Project |
| `remove_project` | Project |
| `save_observation` | Memory |
| `search_across_projects` | Vector / Cross-project |
| `search_logic_flow` | Context |
| `search_memory` | Memory |
| `set_current_project` | Project |
| `submit_lsp_edges` | LSP |
| `unwatch_directory` | Watch |
| `vector_delete_repository` | Vector |
| `vector_index_file` | Vector |
| `vector_index_repository` | Vector |
| `vector_index_status` | Vector |
| `vector_search` | Vector |
| `vector_search_hybrid` | Vector |
| `watch_directory` | Watch |
| `workspace_setup` | Workspace |

## Tool coverage areas

- **Repository/index**: `add_code_to_graph`, `index_status`, `list_indexed_repositories`, `delete_repository`, `get_repository_stats`
- **Search/analysis**: `find_code`, `analyze_code_relationships`, `find_dead_code`, `calculate_cyclomatic_complexity`, `analyze_refactoring`, `find_patterns`, `find_tests`, `execute_cypher_query`
- **Navigation**: `go_to_definition`, `find_all_usages`, `quick_info`
- **Review**: `branch_structural_diff`, `pr_review`
- **Context/impact**: `get_context_capsule`, `get_impact_graph`, `search_logic_flow`, `get_skeleton`, `get_signature`, `explain_result`
- **Cross-project**: `find_similar_across_projects`, `find_shared_dependencies`, `compare_api_surface`, `search_across_projects`
- **Vector**: `vector_index_repository`, `vector_index_file`, `vector_search`, `vector_search_hybrid`, `vector_index_status`, `vector_delete_repository`
- **Watch/jobs**: `watch_directory`, `list_watched_paths`, `unwatch_directory`, `check_job_status`, `list_jobs`
- **Memory**: `save_observation`, `get_session_context`, `search_memory`
- **Project**: `list_projects`, `add_project`, `remove_project`, `set_current_project`, `get_current_project`, `list_branches`, `refresh_project`, `project_status`, `project_sync`, `project_branch_diff`, `project_queue_status`, `project_metrics`
- **Bundle/LSP**: `load_bundle`, `export_bundle`, `submit_lsp_edges`
- **Health**: `check_health`, `diagnose`
- **Workspace**: `workspace_setup`

## MCP prompt and resource

In addition to the 60 tools, `cortex-mcp` registers:

| Type | Identifier | Purpose |
|------|-----------|---------|
| Prompt | `codecortex_route_tools` | Routing guidance prompt for AI agents |
| Resource | `codecortex://guide/tool-routing` | Markdown playbook: tool routing guide |

AI agents can fetch the routing guide via MCP resource reads to understand which tool to use for a given intent.

## Feature flags

Several tools are disabled by default because they are resource-intensive, have persistent side effects, or require additional setup. Enable them via `--enable` CLI args (preferred) or environment variables. Both sources are combined.

### `--enable` (preferred)

```bash
cortex mcp start --enable memory --enable context-capsule --enable impact-graph
```

| `--enable` value | Default | Controls |
|-----------------|---------|---------|
| `context-capsule` | off | `get_context_capsule` |
| `impact-graph` | off | `get_impact_graph`, `analyze_refactoring` |
| `logic-flow` | off | `search_logic_flow` |
| `index-status` | off | `index_status` |
| `skeleton` | off | `get_skeleton`, `get_signature`, `find_tests`, `explain_result`, `find_patterns` |
| `workspace-setup` | off | `workspace_setup` |
| `lsp-ingest` | off | `submit_lsp_edges` |
| `memory` | off | `save_observation`, `get_session_context`, `search_memory` |
| `memory-write` | off | `save_observation` only |
| `memory-read` | off | `get_session_context`, `search_memory` only |

### Environment variables

| Environment variable | Default | Controls |
|---------------------|---------|---------|
| `CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED` | `false` | `get_context_capsule` |
| `CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED` | `false` | `get_impact_graph` |
| `CORTEX_FLAG_MCP_LOGIC_FLOW_ENABLED` | `false` | `search_logic_flow` |
| `CORTEX_FLAG_MCP_INDEX_STATUS_ENABLED` | `false` | `index_status` |
| `CORTEX_FLAG_MCP_SKELETON_ENABLED` | `false` | `get_skeleton` |
| `CORTEX_FLAG_MCP_WORKSPACE_SETUP_ENABLED` | `false` | `workspace_setup` |
| `CORTEX_FLAG_MCP_LSP_INGEST_ENABLED` | `false` | `submit_lsp_edges` |
| `CORTEX_FLAG_MCP_MEMORY_READ_ENABLED` | `false` | `get_session_context`, `search_memory` |
| `CORTEX_FLAG_MCP_MEMORY_WRITE_ENABLED` | `false` | `save_observation` |
| `CORTEX_FLAG_MCP_VECTOR_READ_ENABLED` | `true` | `vector_search`, `vector_search_hybrid`, `search_across_projects` |
| `CORTEX_FLAG_MCP_VECTOR_WRITE_ENABLED` | `true` | `vector_index_repository`, `vector_index_file` |
| `CORTEX_FLAG_MCP_CACHE_ENABLED` | `true` | Query result caching |
| `CORTEX_FLAG_MCP_TELEMETRY_ENABLED` | `true` | Telemetry collection |
| `CORTEX_FLAG_MCP_TFIDF_SCORING_ENABLED` | `true` | TF-IDF reranking |
| `CORTEX_FLAG_MCP_CENTRALITY_SCORING_ENABLED` | `true` | Graph centrality scoring |

Accepted env values: `1`, `true`, `yes`, `on` (enable); `0`, `false`, `no`, `off` (disable).

## Transports

`cortex-mcp` supports four transport modes. All use the same `CortexHandler` tool routing path, so tool contracts and schemas stay consistent.

| Mode | Flag | Default bind | Auth |
|------|------|-------------|------|
| `stdio` | (default) | N/A (local process) | N/A |
| `http-sse` | `--transport http-sse` | `127.0.0.1:3001` | Bearer token |
| `websocket` | `--transport websocket` | `127.0.0.1:3001` | Bearer token |
| `multi` | `--transport multi` | `127.0.0.1:3001` | Bearer token |

Security baseline:
- Default bind is loopback (`127.0.0.1`)
- Non-loopback requires explicit `--allow-remote`
- Optional bearer token auth via `Authorization: Bearer <token>` for HTTP and WebSocket

## Analyzer filter support

Analyzer tools accept path scoping parameters:

| MCP field | CLI equivalent |
|-----------|---------------|
| `include_paths` | `--folder` / `--dir` / `--directory` |
| `include_files` | `--file` |
| `include_globs` | `--include-glob` |
| `exclude_paths` | `--exclude-path` |
| `exclude_files` | `--exclude-file` |
| `exclude_globs` | `--exclude-glob` |

CLI shorthand and MCP payload filters are 1:1.

## Development checks

```bash
cargo test -p cortex-mcp -- --test-threads=1
cargo test -p cortex-mcp --test tool_surface_matrix -- --nocapture
```

## Related docs

- Root usage: [README.md](../../README.md)
- Client integrations: [docs/INTEGRATION.md](../../docs/INTEGRATION.md)
- Feature flags: see `src/flags.rs`
- Test runbook: [docs/INTEGRATION_TEST_MATRIX.md](../../docs/INTEGRATION_TEST_MATRIX.md)
