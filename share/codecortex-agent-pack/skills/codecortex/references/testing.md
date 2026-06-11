# Skill Testing

Validate all three CodeCortex skills with the categories below.

## 1) Triggering tests

### `codecortex`

Expected to trigger:

- "Who calls authenticate in src/auth?"
- "Show blast radius for payment retry changes."
- "Find dead code in src/payments."
- "Go to definition of GraphClient."
- "Hybrid search for where auth tokens are validated."

Expected not to trigger:

- generic writing tasks
- non-code personal questions
- image generation requests

### `codecortex-indexing`

Expected to trigger:

- "The index is stale — how do I reindex?"
- "Set up cortex watch on this repo."
- "Run vector-index before semantic search."
- "What does index_status show?"

Expected not to trigger:

- "Suggest a refactor for this function" (no index lifecycle)
- unrelated DevOps or deployment questions

### `codecortex-workflows`

Expected to trigger:

- "Plan a patch to add token refresh in src/auth."
- "Review my branch against main before merge."
- "What tests should I update for this change?"
- "Is CodeCortex trustworthy right now?"

Expected not to trigger:

- "Write a blog post about Rust"
- pure indexing commands without workflow intent (prefer codecortex-indexing)

## 2) Functional tests

### All skills

1. Preflight (`check_health`, `index_status`) before deep conclusions when using MCP.
2. Respects `freshness` on context tool responses.
3. Uses MCP before CLI except broad smell/refactor scans.

### `codecortex`

1. Applies include/exclude filters on analysis tools.
2. Evidence-first output with scope and findings.
3. Uses `recommend_tools` or router instead of loading full catalog every turn.

### `codecortex-indexing`

1. Recommends `cortex index --force` or `add_code_to_graph` with correct `force` semantics.
2. Mentions incremental-diff deletion → full rebuild rule.
3. Polls jobs for background vector/sync.

### `codecortex-workflows`

1. Calls `get_patch_context` (or equivalent chain) before suggesting broad file reads.
2. Branch review uses `get_delta_context` before impact claims.
3. Blocks high-confidence impact when freshness is stale/partial/unknown.

## 3) Performance tests

Compare against a baseline prompt flow:

- number of tool calls
- completion latency
- retries or failed calls
- tokens consumed

Pass criteria:

- fewer clarification loops
- fewer unscoped scans
- same or better evidence quality

## 4) Verification script (manual)

Run with CodeCortex MCP connected:

| Step | Action | Pass if |
| --- | --- | --- |
| 1 | `check_health` | healthy or actionable errors |
| 2 | `index_status` | status returned for active repo |
| 3 | `recommend_tools` with task "who calls main" | suggests graph/relationship tools |
| 4 | `get_patch_context` with small `budget_tokens` | pack includes freshness + suggested_next_tools |
| 5 | `resources/read` `codecortex://guide/agent-workflows` | markdown guide returned |

Record results in PR or session notes when validating skill changes.

## 5) Cursor hooks and rules (manual)

Prerequisites: `jq` on PATH; [docs/cursor/hooks.json](../../../cursor/hooks.json) symlinked at `.cursor/hooks.json`.

| Step | Action | Pass if |
| --- | --- | --- |
| 1 | Open new agent session in this repo | Hooks output shows `sessionStart` ran |
| 2 | Trigger Write/StrReplace under `crates/` | `preToolUse` suggests `get_patch_context` (advisory) |
| 3 | Task prompt "who calls main" | `postToolUse` suggests `codecortex-analyzer` |
| 4 | `wc -l .cursor/rules/codecortex-*.mdc` | each rule file ≤ 50 lines |
| 5 | `grep -r deny docs/cursor/hooks` | no `permission.*deny` |

Rules: `codecortex-core` and `codecortex-subagents` have `alwaysApply: true`. Index: [RULES-INDEX.md](../../../cursor/RULES-INDEX.md).
