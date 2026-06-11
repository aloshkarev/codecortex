# CodeCortex plugin

Maximize CodeCortex MCP in Claude Code and Cursor: skills, subagents, advisory hooks, rules, and MCP wiring.

## Prerequisites

- `cortex` CLI on PATH (build from this repo or `nix build .#cortex`)
- Graph backend (FalkorDB default): `cortex doctor`
- `jq` for hook scripts

## Claude Code

```bash
cc --plugin-dir /path/to/64-codecortex/plugin/codecortex
```

Includes:

| Component | Path |
| --- | --- |
| Skills | `skills/codecortex*`, `skills/codecortex-setup` |
| Agents | `agents/codecortex-*.md` |
| Hooks | `hooks/hooks.json` (`${CLAUDE_PLUGIN_ROOT}` paths) |
| MCP | `.mcp.json` → `cortex mcp start` |

## Cursor

From your **project** root:

```bash
/path/to/64-codecortex/plugin/codecortex/cursor/install.sh
```

Add MCP per [docs/INTEGRATION.md](../../docs/INTEGRATION.md).

## Discover → Act → Verify

1. **Discover** — `resources/read` → `codecortex://guide/agent-workflows`; `recommend_tools`
2. **Act** — scoped MCP tools, `get_patch_context` before broad reads
3. **Verify** — `check_health`, `index_status`, response `freshness`

## Subagents

| Agent | Use when |
| --- | --- |
| `codecortex-indexer` | Stale index, setup, watch |
| `codecortex-analyzer` | Callers, impact, dead code, search |
| `codecortex-pr-reviewer` | Branch / PR review |
| `codecortex-patch-planner` | Plan edits (read-only) |

## Maintaining the plugin

Canonical sources live in the repo `docs/` tree. After editing skills, agents, or hooks:

```bash
./plugin/codecortex/scripts/sync-from-docs.sh
```

See [AGENTS.md](../../AGENTS.md) and [docs/cursor/RULES-INDEX.md](cursor/RULES-INDEX.md).

## License

Apache-2.0
