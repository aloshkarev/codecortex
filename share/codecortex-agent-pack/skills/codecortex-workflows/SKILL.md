---
name: codecortex-workflows
description: Runs high-value CodeCortex agent playbooks for patch planning, branch and PR review, freshness repair, incident triage, and multi-project scope using context packs and MCP prompts. Use when the user asks to plan a patch before editing, review a branch or PR, triage index trust, repair stale freshness, gather test context for a change, or work across multiple registered projects.
---

# CodeCortex Workflows Skill

*Canonical path: `docs/skills/codecortex-workflows/`; symlinked for Cursor at `.cursor/skills/codecortex-workflows`.*

## Overview

Opinionated playbooks that minimize broad file reads and maximize evidence from bounded context packs.

**Freshness gate (new workspace):** Run `check_health` → `index_status`. If `freshness` is `unknown`, `stale`, or `partial`, complete [codecortex-indexing](../codecortex-indexing/SKILL.md) repair (`add_code_to_graph` or `cortex index --force`) before impact-heavy tools (`get_impact_graph`, `analyze_code_relationships`, `find_dead_code`). Graph-backed analysis on an empty index returns empty results.

**Delegate to subagents:** patch planning → [codecortex-patch-planner](../../agents/codecortex-patch-planner.md); branch/PR review → [codecortex-pr-reviewer](../../agents/codecortex-pr-reviewer.md) ([agents README](../../agents/README.md)).

**Discover:** `resources/read` → `codecortex://guide/agent-workflows` and `codecortex://schema/context-pack`.

**Route analysis questions** (callers, dead code, etc.) to [codecortex](../codecortex/SKILL.md) after workflow context is gathered.

## MCP prompts (when client supports `prompts/list`)

| Prompt | Use for |
| --- | --- |
| `codecortex_patch_plan` | Pre-edit planning |
| `codecortex_branch_review` | Branch / PR review |
| `codecortex_freshness_repair` | Stale or partial index |
| `codecortex_incident_triage` | Trust / health investigation |

Prompts orchestrate the same tool chains below; prefer explicit tools when prompts are unavailable.

## Patch planning

**Goal:** Bounded context before editing.

Checklist:

- [ ] `check_health`
- [ ] `index_status` (acceptable `freshness`)
- [ ] `estimate_context_cost` (optional, large tasks)
- [ ] `get_patch_context` with `task`, `include_paths`, `budget_tokens`, `mode`
- [ ] `get_api_contract` for target symbols
- [ ] `get_test_context` for likely tests
- [ ] `get_skeleton` / `get_signature` only where packs are thin
- [ ] Narrow file reads and implement

Example `get_patch_context`:

```json
{
  "task": "add token refresh to auth client",
  "include_paths": ["src/auth"],
  "budget_tokens": 6000,
  "mode": "feature"
}
```

Read `suggested_next_tools` and `warnings` on the pack response.

## A2A + MCP cooperation

After `cortex_a2a_spawn_session`, read `suggested_next_tools` on the spawn response (or `task.metadata.suggestedNextTools` from GetTask) and call those MCP tools on the host for additional bounded context. Subscribe to task events for **`artifactUpdate`** streams; each cooperation artifact exposes `metadata.mcpToolId`, `metadata.freshness`, and `metadata.suggestedNextTools` outside `Part.data` (extension `https://codecortex.dev/extensions/intelligence-cooperation/v1`).

Example host loop: spawn → subscribe (SSE/WS) → on `artifactUpdate`, call suggested MCP tools → optional final `cortex_a2a_get_task` with `include_artifacts: true`.

## A2A consensus review

**Goal:** Cross-verify patches inside CodeCortex before returning a compact result to the host.

Configure `~/.cortex/config.toml` (`[a2a]`, `mcp.tools.a2a_spawn_session`). **Do not** use environment variables for A2A.

Workflow loop:

1. `codecortex-patch-planner` (or hub stub) builds context from `get_patch_context`.
2. Planner sends `CodeInsight` to `codecortex-analyzer` over the A2A bus (not via host MCP).
3. Analyzer runs `get_impact_graph` (in-process) and posts to the graph blackboard.
4. On contract or lock-order issues, analyzer sends `Reject` to planner; host is not involved.
5. Optional: `codecortex-validator` runs `cargo check` via external A2A client.
6. Repeat until `Accept`, then return `FinalResult` through **`cortex_a2a_spawn_session`** only.

MCP entry (`cortex_a2a_spawn_session`, `cortex_a2a_get_task` with `spec_json: true`, `cortex_a2a_cancel_task`, `cortex_a2a_list_tasks`, `cortex_a2a_send_message`, `cortex_a2a_subscribe_task`). Prompt: **`codecortex_a2a_consensus`**.

```json
{
  "task": "Fix deadlock in src/transport.rs",
  "workflow": "consensus_review",
  "include_paths": ["src/transport.rs"],
  "return_immediately": true
}
```

Other workflows: `patch_plan` (planner + validator), `impact_review` (analyzer-only), `pr_review` (delta + reviewer + optional analyzer). Honor spawn `warnings` when freshness is not fresh.

### Indexed-codebase scenarios (A2A)

| Scenario | Workflow | Key spawn fields |
| --- | --- | --- |
| Pre-edit patch plan | `patch_plan` | `task`, `include_paths`, `mode`, `budget_tokens` |
| Blast-radius only | `impact_review` | `target_symbol`, `include_paths` |
| Branch / PR review | `pr_review` | `source_branch`, `target_branch`, `include_paths` |
| Multi-role consensus | `consensus_review` | `include_paths`; add `demo_fixture` only for deadlock demo |
| Stale index warning | any | spawn `freshness` + `index_status` before high-confidence claims |

Spawn once; poll `cortex_a2a_get_task` with `spec_json: true` for capsule artifacts in task history.

## Branch / PR review

**Goal:** Structural impact of a branch without reading the whole diff blindly.

Checklist:

- [ ] `index_status`
- [ ] `get_delta_context` (`source_branch`, `target_branch`, `budget_tokens`)
- [ ] `branch_structural_diff`
- [ ] `get_impact_graph` on high-risk symbols from delta
- [ ] `get_test_context`
- [ ] `pr_review` when PR-style diff input is available

Do not claim full blast radius if `freshness` is `stale`, `partial`, or `unknown`.

## PR review (A2A workflow)

**Goal:** Merge-oriented branch review inside the A2A hub without chaining many MCP tools in the host.

Configure `~/.cortex/config.toml` (`[a2a]`, `mcp.tools.a2a_spawn_session`). **Do not** use environment variables for A2A.

Workflow loop:

1. Hub builds delta context via `get_patch_context` (capsule URI in delegation).
2. Hub delegates `TaskDelegation` to `pr_reviewer` (in-process or external per role `mode`).
3. `PrReviewerRunner` returns `CodeInsight` with blast-radius summary; may `Accept` / `Reject` on critical risk.
4. Optional: hub forwards insight to `analyzer` for impact confirmation when analyzer is enabled.
5. Hub records `FinalResult`; host polls **`cortex_a2a_spawn_session`** only.

MCP entry (`cortex_a2a_spawn_session` with `workflow: "pr_review"`). Same poll/subscribe tools as consensus review.

```json
{
  "task": "Review feature/auth-refresh before merge",
  "workflow": "pr_review",
  "include_paths": ["src/auth"],
  "return_immediately": true
}
```

**External roles:** When `[a2a.roles.pr_reviewer].mode = "external"`, set a reachable `agent_card_url`; hub collects replies via GetTask/SSE polling (`runtime/external.rs`). Missing endpoints yield no replies — not a silent in-process fallback unless `[a2a].force_in_process = true`.

**`force_in_process`:** Default `false` — each step uses the role's configured `mode`. Set `force_in_process = true` to run every role in-process (local tests or all-local deployments).

Honor spawn `warnings` when freshness is not fresh.

## Freshness repair

**Goal:** Restore trustworthy graph/vector state.

Checklist:

- [ ] `explain_index_freshness`
- [ ] `diagnose`
- [ ] `index_status`, `vector_index_status`
- [ ] Reindex per [codecortex-indexing](../codecortex-indexing/SKILL.md)
- [ ] Poll `list_jobs` / `check_job_status` for background work
- [ ] Re-run `check_health` and confirm `freshness` on a spot-check context tool

## Incident / trust triage

**Goal:** Decide whether CodeCortex answers are safe to use right now.

Checklist:

- [ ] `check_health`
- [ ] `index_status`
- [ ] `diagnose`
- [ ] Summarize: healthy / degraded / blocked
- [ ] If degraded: freshness repair workflow; **no high-confidence impact claims** until repaired

## Multi-project

**Goal:** Correct repo scope before tools assume a single tree.

1. `list_projects`
2. `get_current_project` or `set_current_project`
3. Use `--project` / project-scoped filters on tools
4. Cross-repo: `find_similar_across_projects`, `find_shared_dependencies`, `compare_api_surface`, `search_across_projects`

Outside known projects, `find` / search default to all-project scope unless narrowed.

## Full-stack agents (repo + backend)

When the user also runs a backend-for-agents MCP (e.g. InsForge):

- CodeCortex: structure, tests, patch/delta context, call graphs.
- Backend MCP: schema, auth, storage, gateway, deployment.

Read `codecortex://guide/agent-platforms` — do not substitute one for the other.

## Output contract

Same as codecortex: Scope, Findings (with freshness caveat), Next Actions.

## Progressive disclosure

- [references/context-pack.md](references/context-pack.md)
- [references/mcp-client.md](references/mcp-client.md)
- Index repair: [codecortex-indexing](../codecortex-indexing/SKILL.md)
