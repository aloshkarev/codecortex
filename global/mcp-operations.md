# MCP Operations Policy

- Use only approved MCP servers from `mcp/servers.toml`.
- Required MCP baseline: `context7`, `docker_mcp`, and code index provider (`codecortex` by default, `sdl_mcp` fallback).
- Keep server timeouts and retries within policy limits.
- Preserve user-defined server sections when merging MCP config.
- Validate server health before depending on MCP-only workflows.
- Prefer deterministic, read-only operations in CI checks.
