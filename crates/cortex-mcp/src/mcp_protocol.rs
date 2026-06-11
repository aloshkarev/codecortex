//! Static MCP / rmcp compatibility notes for the `codecortex://guide/mcp-protocol` resource.

/// Markdown body: supported surface, library version source, and known gaps.
pub fn mcp_protocol_guide() -> String {
    format!(
        "# CodeCortex MCP protocol notes\n\n\
## Stack\n\n\
- Rust MCP server uses the **rmcp** crate (workspace version; see root `Cargo.toml`).\n\
- Capabilities declared in `CortexHandler::get_info`: tools, resources, resource subscriptions, prompts.\n\
- Transports: stdio (default), streamable HTTP / WebSocket via `network_server` (see `cortex mcp start`).\n\n\
## Supported client flows\n\n\
| MCP area | Server behavior |\n\
| --- | --- |\n\
| `tools/list`, `tools/call` | All `#[tool]` methods on `CortexHandler` via `tool_router!` |\n\
| `resources/list`, `resources/read` | Static guides (`codecortex://guide/*`, `codecortex://tools/catalog`) |\n\
| `resources/subscribe` | Acknowledged as no-op for static resources (avoids client noise) |\n\
| `prompts/list`, `prompts/get` | `codecortex_*` workflow prompts |\n\n\
## Intentional limitations\n\n\
- **Roots / `roots/list`**: not implemented; repository scope is driven by CodeCortex projects, \
`repo_path` parameters, and indexer config—not MCP workspace roots.\n\
- **Sampling**: not implemented; use your agent host for model sampling.\n\
- **OAuth on MCP**: network mode uses optional static **Bearer** token (`--token-env`); \
full OAuth device flows belong at the reverse proxy / gateway.\n\n\
## Operational knobs\n\n\
- `CORTEX_MCP_PROFILE=strict`: tighter feature defaults (see `mcp_profile` module).\n\
- `CORTEX_MCP_AUDIT_LOG=/path/to/audit.jsonl`: one JSON line per audited tool response (envelope tools).\n\
- Per-tool feature flags: `CORTEX_FLAG_MCP_<NAME>_ENABLED` (see `FeatureFlags`).\n"
    )
}
