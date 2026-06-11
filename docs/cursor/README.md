# Cursor hooks (CodeCortex)

Canonical hook configuration for this repository. For **other projects**, use the distribution pack: [plugin/codecortex/cursor/](../../plugin/codecortex/cursor/) (`install.sh` copies hooks and rules into `.cursor/`).

Cursor loads this repo via symlinks:

- [`.cursor/hooks.json`](../../.cursor/hooks.json) → `docs/cursor/hooks.json`
- [`.cursor/hooks/`](../../.cursor/hooks/) → `docs/cursor/hooks/`

## Prerequisites

- `jq` on PATH (`command -v jq`)
- **user-codecortex** MCP enabled (`cortex mcp start`)

## Behavior

All hooks are **advisory only**: they inject context and suggestions. They never block writes, shell, or MCP (`permission: deny` is not used).

| Event | Script | Purpose |
| --- | --- | --- |
| `sessionStart` | `session-codecortex-context.sh` | Inject discover→act→verify + skill/subagent pointers |
| `subagentStart` | `subagent-codecortex-inject.sh` | Bind CodeCortex subagents to skills |
| `preToolUse` | `preflight-before-edit.sh` | Suggest `get_patch_context` for `crates/` edits |
| `postToolUse` | `after-task-codecortex.sh` | Suggest subagent for matching Task prompts |
| `subagentStop` | `subagent-codecortex-followup.sh` | Follow up on blocked freshness |
| `beforeMCPExecution` | `mcp-codecortex-reminder.sh` | Nudge toward CodeCortex for structure queries |

## Debugging

1. Save `hooks.json` (Cursor reloads on save).
2. Cursor **Hooks** settings tab or **Hooks** output channel.
3. Restart Cursor if hooks do not load.

## Related

- Rules: [`.cursor/rules/`](../../.cursor/rules/)
- Sync index: [RULES-INDEX.md](RULES-INDEX.md)
- Skills / agents: [AGENTS.md](../../AGENTS.md)
