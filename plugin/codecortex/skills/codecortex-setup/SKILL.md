---
name: codecortex-setup
description: Bootstraps CodeCortex for the current project — cortex doctor, graph index, optional vector index, and MCP health checks. Use when the user asks to set up codecortex, bootstrap code intelligence, connect cortex MCP, or prepare the repo for graph-backed analysis.
disable-model-invocation: false
---

# CodeCortex setup

User-invoked bootstrap for CodeCortex on the active repository.

## Prerequisites check

1. Confirm `cortex` is on PATH (`cortex --version` or `which cortex`).
2. Confirm `jq` is available if using project hooks.

## Bootstrap steps

Run in order (Shell):

```bash
cortex doctor
cortex index "$(pwd)" --force
cortex vector-index "$(pwd)"   # optional; needed for vector_search_hybrid
```

## MCP verification

If **codecortex** / **user-codecortex** MCP is connected:

1. `manage_codecortex` with `action=assess` (or `check_health` + `index_status`)
2. First-time project assets: `manage_codecortex` with `action=bootstrap`, `install_agent_pack=true` — or `workspace_setup` with `install_agent_pack=true`, `generate_configs=true`
3. Optional keep-warm index: `enable_watch=true` on either tool
4. If `freshness` is stale/unknown, run indexing steps above or delegate **codecortex-indexer** subagent

Read MCP resource `codecortex://guide/agent-pack-bootstrap` for env vars and install paths.

## After setup

Point the user to packaged capabilities:

| Need | Skill / agent |
| --- | --- |
| Analysis | `codecortex` skill / `codecortex-analyzer` agent |
| Index repair | `codecortex-indexing` / `codecortex-indexer` |
| Patch plan | `codecortex-workflows` / `codecortex-patch-planner` |
| PR review | `codecortex-workflows` / `codecortex-pr-reviewer` |

Plugin root: `${CLAUDE_PLUGIN_ROOT}` when installed via Claude Code plugin.

## Cursor users

Run `cursor/install.sh` from this plugin to install skills, agents, hooks, rules, and `.cursor/mcp.json` (use `--symlink` when the pack lives inside the same git repo).

**Sync:** Run `plugin/codecortex/scripts/sync-from-docs.sh` after changing canonical docs in the repo.
