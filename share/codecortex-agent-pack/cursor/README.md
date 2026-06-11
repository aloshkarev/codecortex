# CodeCortex Cursor install pack

Copies advisory hooks and rules into a project's `.cursor/` directory.

## Install

From the **target project** root:

```bash
/path/to/64-codecortex/plugin/codecortex/cursor/install.sh
```

## MCP

Add to `~/.cursor/mcp.json`:

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

## Contents

- `hooks.json` + `hooks/*.sh` — same behavior as repo `docs/cursor/`
- `rules/*.mdc` — six codecortex rules
- `RULES-INDEX.md` — intent → skill → subagent map

Hooks are advisory only (no blocking).
