# Integrations

> This guide explains how to connect MCP-capable AI clients to CodeCortex and how to configure the MCP server for different deployment scenarios.

For the multi-language real integration test runbook, see [docs/INTEGRATION_TEST_MATRIX.md](INTEGRATION_TEST_MATRIX.md).

Runtime language support: Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, Shell (14 languages).

## Recommended runtime flow

Use one shared runtime per repository:

```bash
# 1. Index graph data
cortex index /path/to/repo

# 2. Index vector data (optional, enables semantic search)
cortex vector-index /path/to/repo

# 3. Start MCP server
cortex mcp start
```

One-command bootstrap:

```bash
cortex doctor && cortex index "/path/to/repo" && cortex vector-index "/path/to/repo" && cortex mcp start
```

## Scope and context model

CodeCortex uses two default scopes:

| Context | Default scope |
|---------|--------------|
| Inside a known project (registered via `cortex project add`) | Single-project, branch-aware graph context |
| Outside known projects | `find`/`search` default to all-project scope |

Override flags:
- `--all-projects` — force cross-project mode
- `--project <path>` — force single-project mode

## MCP transports

The default transport is stdio. All transports use the same `CortexHandler` routing path, so tool behavior and schemas are identical.

```bash
cortex mcp start                          # stdio (default)
```

Network options:

| Flag | Description |
|------|-------------|
| `--transport http-sse` | HTTP+SSE responses on `POST /mcp` |
| `--transport websocket` | JSON-RPC over `GET /ws` |
| `--transport multi` | Expose both HTTP+SSE and WebSocket simultaneously |
| `--listen <addr:port>` | Bind address (default `127.0.0.1:3001`) |
| `--allow-remote` | Required for non-loopback bind |
| `--token <value>` | Static bearer token |
| `--token-env <ENV>` | Bearer token from environment variable |
| `--max-clients <N>` | Maximum concurrent network clients |
| `--idle-timeout-secs <N>` | Disconnect idle clients after N seconds |

## Client setup

### Cursor

`~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "cortex",
      "args": ["mcp", "start"]
    }
  }
}
```

### VS Code (with MCP-capable extension)

`.vscode/mcp.json` or user-level MCP config:

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "cortex",
      "args": ["mcp", "start"]
    }
  }
}
```

For network transport (e.g., remote dev container):

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "cortex",
      "args": ["mcp", "start", "--transport", "http-sse", "--listen", "127.0.0.1:3001"]
    }
  }
}
```

### Claude Code

```bash
claude mcp add cortex -- cortex mcp start
```

### Codex CLI

```bash
codex mcp add cortex -- cortex mcp start
```

### Gemini CLI

`~/.gemini/settings.json`:

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "cortex",
      "args": ["mcp", "start"]
    }
  }
}
```

### Zed

```json
{
  "context_servers": {
    "codecortex": {
      "source": "custom",
      "command": "cortex",
      "args": ["mcp", "start"],
      "env": {}
    }
  }
}
```

### Neovim

Configure your MCP-capable plugin to run:

```bash
cortex mcp start
```

## Feature flags

Several MCP tools are disabled by default because they are resource-intensive, have persistent side effects, or require additional setup. Enable them with `--enable` CLI args (preferred) or environment variables. Both sources are combined — either can activate a tool.

### `--enable` flags (preferred — no env vars needed)

```bash
cortex mcp start --enable memory --enable context-capsule
cortex mcp start --enable memory --enable impact-graph --enable skeleton
```

| `--enable` value | Default | Tool(s) controlled |
|-----------------|---------|-------------------|
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

For Cursor, pass `--enable` in `args`:

```json
{
  "mcpServers": {
    "codecortex": {
      "command": "cortex",
      "args": ["mcp", "start", "--enable", "memory", "--enable", "context-capsule"]
    }
  }
}
```

### Environment variable overrides

Tools can also be toggled with environment variables. These are combined with `--enable` args.

| Tool | Environment variable | Default |
|------|---------------------|---------|
| `get_context_capsule` | `CORTEX_FLAG_MCP_CONTEXT_CAPSULE_ENABLED` | `false` |
| `get_impact_graph` | `CORTEX_FLAG_MCP_IMPACT_GRAPH_ENABLED` | `false` |
| `search_logic_flow` | `CORTEX_FLAG_MCP_LOGIC_FLOW_ENABLED` | `false` |
| `index_status` | `CORTEX_FLAG_MCP_INDEX_STATUS_ENABLED` | `false` |
| `get_skeleton` | `CORTEX_FLAG_MCP_SKELETON_ENABLED` | `false` |
| `workspace_setup` | `CORTEX_FLAG_MCP_WORKSPACE_SETUP_ENABLED` | `false` |
| `submit_lsp_edges` | `CORTEX_FLAG_MCP_LSP_INGEST_ENABLED` | `false` |
| `get_session_context`, `search_memory` | `CORTEX_FLAG_MCP_MEMORY_READ_ENABLED` | `false` |
| `save_observation` | `CORTEX_FLAG_MCP_MEMORY_WRITE_ENABLED` | `false` |
| `vector_search`, `vector_search_hybrid`, `search_across_projects` | `CORTEX_FLAG_MCP_VECTOR_READ_ENABLED` | `true` |
| `vector_index_repository`, `vector_index_file` | `CORTEX_FLAG_MCP_VECTOR_WRITE_ENABLED` | `true` |
| Query result caching | `CORTEX_FLAG_MCP_CACHE_ENABLED` | `true` |
| Telemetry | `CORTEX_FLAG_MCP_TELEMETRY_ENABLED` | `true` |
| TF-IDF reranking | `CORTEX_FLAG_MCP_TFIDF_SCORING_ENABLED` | `true` |
| Graph centrality scoring | `CORTEX_FLAG_MCP_CENTRALITY_SCORING_ENABLED` | `true` |

Accepted env values: `1`, `true`, `yes`, `on` (enable); `0`, `false`, `no`, `off` (disable).

## How MCP requests map to data

| Tool area | Data source |
|-----------|------------|
| Graph tools (`find_code`, `analyze_*`, navigation) | Memgraph/Neo4j/Neptune graph |
| Vector tools (`vector_search`, `vector_search_hybrid`) | LanceDB/Qdrant vector index |
| Hybrid paths (`get_context_capsule`, `search_logic_flow`) | Graph + vector combined |
| Memory tools | SQLite session store (`~/.cortex/memory.db`) |

Typical request lifecycle:

1. AI client discovers tools via `tools/list`
2. AI client calls a tool with arguments
3. `CortexHandler` routes to the appropriate crate
4. Tool executes on indexed data
5. JSON response returns to the client

## Navigation and review tools

Navigation tools require indexed graph data with resolved navigation edges:

| Tool | Requires |
|------|---------|
| `go_to_definition` | Indexed repo with `MEMBER_OF` + `TYPE_REFERENCE` edges |
| `find_all_usages` | Indexed repo with `CALLS` + `TYPE_REFERENCE` edges |
| `quick_info` | Indexed repo |
| `branch_structural_diff` | Both branches indexed for best results |
| `pr_review` | Indexed repo or local diff input |

CLI equivalents:

```bash
cortex goto "GraphClient::connect"
cortex usages "GraphClient"
cortex info "GraphClient"
cortex analyze branch-diff feature/nav main --structural
```

## Analyze scope filters

CLI flags and MCP payload fields are 1:1:

| CLI flag | MCP field |
|----------|-----------|
| `--folder` / `--dir` / `--directory` | `include_paths` |
| `--file` | `include_files` |
| `--include-glob` | `include_globs` |
| `--exclude-path` | `exclude_paths` |
| `--exclude-file` | `exclude_files` |
| `--exclude-glob` | `exclude_globs` |

## Network security hardening

For any network transport deployment:

- **Default bind is loopback** (`127.0.0.1`). Non-loopback requires `--allow-remote`.
- **Always set a bearer token** via `--token-env CORTEX_MCP_TOKEN`.
- **Terminate TLS at a reverse proxy** (nginx, Caddy, Traefik). CodeCortex does not terminate TLS itself.
- **Rotate tokens periodically** and use `--token-env` to avoid tokens in shell history or process lists.

Example production-safe remote setup (behind nginx TLS proxy):

```bash
CORTEX_MCP_TOKEN="$(openssl rand -hex 32)" \
cortex mcp start \
  --transport multi \
  --listen 127.0.0.1:3001 \
  --token-env CORTEX_MCP_TOKEN
```

Then configure nginx to proxy `https://your.host/mcp` → `http://127.0.0.1:3001/mcp` with TLS.

See [SECURITY.md](../SECURITY.md) for the full security model.

## Verification checklist

1. `cortex doctor` passes
2. Repository is indexed (`cortex list` shows the repo)
3. `cortex mcp tools` returns the tool list
4. One symbol lookup succeeds: `cortex find name main`
5. One analysis query succeeds: `cortex analyze callers main`

## Troubleshooting

| Symptom | Solution |
|---------|----------|
| No tools visible in AI client | Verify `command` path and `args` in client config. Run `cortex mcp tools` manually. |
| Empty results | Ensure the repo is indexed in the same runtime session. Run `cortex list`. |
| Stale results after code changes | Re-index with `cortex index /path --force` and restart the MCP process. |
| Backend problems | Run `cortex doctor` and inspect service health. |
| Memory/capsule tools not available | Set `CORTEX_FLAG_MCP_MEMORY_READ_ENABLED=true` and related flags. |
| Network transport rejected | Check `--allow-remote` and bearer token configuration. See [SECURITY.md](../SECURITY.md). |
