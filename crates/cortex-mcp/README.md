# cortex-mcp

`cortex-mcp` exposes CodeCortex functionality as MCP tools.

It receives MCP JSON-RPC requests and routes them into graph, analyzer, vector, project, and memory operations.

Language coverage follows runtime parser/indexer support (including Kotlin, Swift, JSON, and Shell).

## Tool coverage areas

- repository/index
- search/analysis
- context/impact
- vector
- project
- watch/jobs
- memory
- bundle/LSP
- advanced queries

## Recent updates

- Added cross-project tools:
  - `find_similar_across_projects`
  - `find_shared_dependencies`
  - `compare_api_surface`
  - `search_across_projects`
- Added navigation and review tools:
  - `go_to_definition`
  - `find_all_usages`
  - `quick_info`
  - `branch_structural_diff`
  - `pr_review`
- `pr_review` now supports diff-aware local input loading for branch/PR-style review scenarios.

## Serve transports

`cortex-mcp` now supports:

- `stdio` (default, backward-compatible)
- `http-sse` via `POST /mcp` (SSE-framed MCP responses)
- `websocket` via `GET /ws`
- `multi` to expose both network endpoints together

All transports route through the same `CortexHandler` RMCP tool path (stdio, HTTP+SSE, and WebSocket), so tool behavior and schemas stay consistent across clients.

Security baseline:

- default bind is loopback (`127.0.0.1`)
- non-loopback requires explicit `--allow-remote`
- optional bearer token auth (`Authorization: Bearer <token>`) for HTTP and WebSocket

## Filter support

Analyzer tools support:

- `include_paths`
- `include_files`
- `include_globs`
- `exclude_paths`
- `exclude_files`
- `exclude_globs`

CLI shorthand parity:

- CLI `--folder` (also `--dir`, `--directory`) maps to MCP `include_paths`
- CLI `--file` maps to MCP `include_files`

This keeps MCP payload filters and CLI analyze scope guidance 1:1.

## Development checks

```bash
cargo test -p cortex-mcp -- --test-threads=1
cargo test -p cortex-mcp --test tool_surface_matrix -- --nocapture
```

## Related docs

- root usage: `README.md`
- client integrations: `docs/INTEGRATION.md`
- test runbook: `docs/INTEGRATION_TEST_MATRIX.md`
