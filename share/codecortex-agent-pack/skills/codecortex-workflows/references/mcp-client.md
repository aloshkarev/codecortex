# MCP client setup

## Cursor

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

Optional feature flags:

```bash
cortex mcp start --enable memory --enable context-capsule --enable index-status
```

Useful `--enable` values: `memory`, `memory-read`, `memory-write`, `context-capsule`, `impact-graph`, `logic-flow`, `index-status`, `skeleton`, `workspace-setup`, `lsp-ingest`.

Tools are on by default; opt out with `CORTEX_FLAG_MCP_*_ENABLED=0`. `--enable` on `cortex mcp start` forces groups back on.

## Transports

| Transport | Command |
| --- | --- |
| stdio (default) | `cortex mcp start` |
| HTTP+SSE | `cortex mcp start --transport http-sse --listen 127.0.0.1:3001` |
| WebSocket | `cortex mcp start --transport websocket --listen 127.0.0.1:3001` |
| Both network | `cortex mcp start --transport multi --listen 127.0.0.1:3001` |

Remote bind requires `--allow-remote`. Optional auth: `--token` or `--token-env CORTEX_MCP_TOKEN`.

All transports share the same `CortexHandler` tool surface.

## Discovery after connect

1. `resources/list` → read `codecortex://guide/agent-workflows`
2. `prompts/list` → patch / review / freshness / triage prompts
3. `check_health` → `index_status`
4. `tools/list` or `recommend_tools` for routing

Full catalog: `codecortex://tools/catalog` or `cortex mcp tools --metadata`.

## Bootstrap per repository

```bash
cortex doctor && cortex index "<repo>" && cortex vector-index "<repo>" && cortex mcp start
```

Vector step optional unless using hybrid or semantic search.

## InsForge / backend platforms

A model gateway or backend MCP does not replace CodeCortex indexing. For tasks that mix repo structure and live deployed state, read `codecortex://guide/agent-platforms`.

More clients: `docs/INTEGRATION.md`.
