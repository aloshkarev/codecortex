# CodeCortex MCP Integrations

This guide provides practical one-line and production-grade integration paths for:

- Cursor
- Neovim
- Zed
- Claude Code
- Codex CLI
- Gemini CLI

It assumes `cortex` is installed and available in `PATH`.

## Recommended Runtime Model

Use one shared local runtime for all clients:

1. index graph data (`cortex index`)
2. index vector data (`cortex vector-index`)
3. expose MCP server (`cortex mcp start`)

This avoids per-client indexing drift and keeps tool results consistent.

## Universal One-Line Bootstrap

Replace `<repo>` with your repository path:

```bash
cortex doctor && cortex index "<repo>" && cortex vector-index "<repo>" && cortex mcp start
```

## Cursor

### Cursor one-line integration

```bash
mkdir -p ~/.cursor && printf '%s\n' '{' '  "mcpServers": {' '    "codecortex": {' '      "command": "cortex",' '      "args": ["mcp", "start"]' '    }' '  }' '}' > ~/.cursor/mcp.json
```

### Cursor production setup

- keep `~/.cursor/mcp.json` under dotfiles management
- add env vars as needed in the server block
- keep indexing in a separate terminal (watch/daemon), not on every prompt

## Neovim

### Neovim one-line integration

```bash
cortex mcp start
```

Then connect your MCP-capable Neovim AI plugin/client to stdio command `cortex mcp start`.

### Neovim production setup

- add a Neovim command wrapper (for example `:CortexMcpStart`) that runs `cortex mcp start`
- use one shared config per project for repository path and indexing policy

## Zed

### Zed one-line integration

Add this server in Zed settings (`context_servers`):

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

### Zed production setup

- store team-default server profile in docs
- validate server status in Agent Panel after startup

## Claude Code

### Claude Code one-line integration

```bash
claude mcp add cortex -- cortex mcp start
```

### Claude Code production setup

- use project scope for shared repositories
- limit tool surface where needed for safety and focus

## Codex CLI

### Codex CLI one-line integration

```bash
codex mcp add cortex -- cortex mcp start
```

### Codex CLI production setup

- configure `.codex/config.toml` for enabled/disabled tools and timeouts
- verify active servers via `codex mcp list`

## Gemini CLI

### Gemini CLI one-line integration

```bash
mkdir -p ~/.gemini && printf '%s\n' '{' '  "mcpServers": {' '    "codecortex": {' '      "command": "cortex",' '      "args": ["mcp", "start"],' '      "env": {}' '    }' '  }' '}' > ~/.gemini/settings.json
```

### Gemini CLI production setup

- prefer project-level `.gemini/settings.json` for team repos
- use allowlist/exclude MCP controls if required by policy

## Efficient Usage Patterns

1. Keep index fresh in background:
   - `cortex watch <repo>` or daemon-based flow
2. Use two-stage retrieval:
   - graph tools for structure/dependencies
   - vector tools for semantic expansion
   - direct MCP vector tools when available:
     - `vector_index_repository`, `vector_index_file`
     - `vector_search`, `vector_search_hybrid`
     - `vector_index_status`, `vector_delete_repository`
3. Use task profiles:
   - debug, refactor, review, onboarding
4. Control noise and cost:
   - branch-scoped indexing
   - include/exclude policy
   - depth limits for graph traversals

## Validation Matrix

For each platform, validate these 5 scenarios:

1. MCP server connection is active
2. `tools/list` returns CodeCortex tools
3. symbol lookup returns expected entity
4. impact/call-chain query works on indexed repo
5. diagnostics/health tool returns healthy status

## Runbook

### Start sequence

```bash
cortex doctor
cortex index "<repo>"
cortex vector-index "<repo>"
cortex mcp start
```

### Update sequence

```bash
cortex index "<repo>"
cortex vector-index "<repo>"
```

### Troubleshooting

- server not visible: verify client config path and command
- no results: verify indexing completed for target repo
- stale results: re-run index and vector-index, then restart MCP server
- backend failure: run `cortex doctor` and inspect Memgraph/vector store status
