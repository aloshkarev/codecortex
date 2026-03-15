# Integrations

This guide explains how to connect MCP-capable clients to CodeCortex.

For the multi-language real integration test runbook, see:

- `docs/INTEGRATION_TEST_MATRIX.md`

Runtime language support includes Rust, Python, Go, TypeScript, JavaScript, C, C++, Java, PHP, Ruby, Kotlin, Swift, JSON, and Shell.

## Recommended runtime flow

Use one shared runtime per repository:

1. index graph data (`cortex index`)
2. index vector data (`cortex vector-index`)
3. run MCP server (`cortex mcp start`)

## Bootstrap command

```bash
cortex doctor && cortex index "<repo>" && cortex vector-index "<repo>" && cortex mcp start
```

## MCP transports

Default behavior remains stdio:

```bash
cortex mcp start
```

Network options:

- `--transport http-sse` for Streamable HTTP-style SSE responses on `POST /mcp`
- `--transport websocket` for JSON-RPC over `GET /ws`
- `--transport multi` to expose both endpoints
- `--listen <addr:port>` to set bind address (default `127.0.0.1:3001`)
- `--allow-remote` required for non-loopback bind
- `--token` or `--token-env` for optional bearer token auth

All transports use the same `CortexHandler` routing path, so tool contracts and behavior remain aligned between stdio and network clients.

Example remote-safe setup:

```bash
cortex mcp start \
  --transport multi \
  --listen 0.0.0.0:3001 \
  --allow-remote \
  --token-env CORTEX_MCP_TOKEN
```

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

## Claude Code

```bash
claude mcp add cortex -- cortex mcp start
```

## Codex CLI

```bash
codex mcp add cortex -- cortex mcp start
```

## Gemini CLI

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

## Zed

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

## Neovim

Configure your MCP-capable plugin/client to run:

```bash
cortex mcp start
```

## How MCP requests map to data

- graph tools query Memgraph/Neo4j-backed code graph
- vector tools query vector index data
- hybrid paths combine both

Typical request lifecycle:

1. client discovers tools via `tools/list`
2. client calls a tool with arguments
3. tool executes on indexed data
4. JSON response returns to the client

## Analyze scope filters

CLI flags:

- `--file` (same as include-file)
- `--folder` (same as include-path; aliases: `--dir`, `--directory`)
- `--include-path`
- `--include-file`
- `--include-glob`
- `--exclude-path`
- `--exclude-file`
- `--exclude-glob`

MCP fields:

- `include_paths`
- `include_files`
- `include_globs`
- `exclude_paths`
- `exclude_files`
- `exclude_globs`

CLI shorthand to MCP mapping:

- `--folder` / `--dir` / `--directory` -> `include_paths`
- `--file` -> `include_files`

## Verification checklist

1. `cortex doctor` passes
2. repository is indexed
3. `cortex mcp tools` returns tools
4. one symbol lookup succeeds
5. one impact/relationship query succeeds

## Troubleshooting

- no tools visible: verify command path and args
- empty results: ensure repo was indexed in same runtime
- stale output: re-index and restart MCP process
- backend problems: run `cortex doctor` and inspect services

For remote exposure, use TLS termination/reverse proxy in front of CodeCortex and keep bearer token auth enabled.
