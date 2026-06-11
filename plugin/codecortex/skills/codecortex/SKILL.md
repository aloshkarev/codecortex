---
name: codecortex
description: Routes code intelligence tasks to the best CodeCortex MCP tool using scoped filters and cost-aware choices. Use when users ask who calls what, impact or path tracing, dead code, complexity hotspots, design patterns, refactoring analysis, go to definition, usages, vector or hybrid search, patch context, delta context, API contract, test context, or whether MCP/index health is trustworthy. Trigger phrases include callers, callees, blast radius, call chain, dead code, complexity, pattern detection, refactor suggestions, check health, index status, get_patch_context, and get_delta_context. Prefer MCP tools first; use cortex analyze CLI only for broad file smell or refactoring scans.
---

# CodeCortex Skill

*Canonical path: `docs/skills/codecortex/`; symlinked for Cursor at `.cursor/skills/codecortex`.*

## Overview

Context-aware recipe for CodeCortex MCP and analyze workflows with low token cost and high evidence quality.

**Semantic layer:** discover via MCP resources and `recommend_tools`, act through scoped bounded tools, verify with `check_health` / `index_status` and response `freshness`.

**With InsForge (or similar):** load `codecortex://guide/agent-platforms` when the task touches both repository and live backend. Route repo questions to CodeCortex; route operational backend questions to the backend MCP.

**Related skills:** indexing lifecycle → [codecortex-indexing](../codecortex-indexing/SKILL.md); patch/review playbooks → [codecortex-workflows](../codecortex-workflows/SKILL.md).

**Delegate to subagents** (long MCP chains): [codecortex-analyzer](../../agents/codecortex-analyzer.md) for tracing/impact; see [docs/agents/README.md](../../agents/README.md).

Execution model:

- MCP tools are default.
- CLI is fallback for broad filesystem smell and refactoring scans.
- Always scope first, then analyze.

## Discover → Act → Verify

1. **Discover** — `resources/read` on `codecortex://guide/agent-workflows` (and `codecortex://guide/tool-routing` when routing is unclear). Use `recommend_tools` instead of loading the full catalog every turn. Use `get_tool_guidance` before an unfamiliar expensive tool.
2. **Act** — call scoped tools with filters and `budget_tokens`; prefer context packs over ad-hoc full-file reads.
3. **Verify** — `check_health`, `index_status`, and response `freshness` / `source_policy` before high-confidence impact or safety claims.

## Index tier gate

Before graph or vector tools, confirm `index_status` satisfies the tool's minimum tier:

| Tier | Meaning |
| --- | --- |
| `none` | No graph required |
| `project` | Project registry / branch metadata |
| `graph` | Graph index for repository |
| `vector` | Vector index |
| `graph_and_vector` | Both (e.g. `vector_search_hybrid`) |

If tier is unmet, run indexing per [codecortex-indexing](../codecortex-indexing/SKILL.md) before retrying.

## Cost discipline

| Class | When to use | Examples |
| --- | --- | --- |
| `cheap` | Preflight, routing, light metadata | `check_health`, `index_status`, `recommend_tools`, `go_to_definition` |
| `bounded` | Token-capped context or search | `get_patch_context`, `find_code`, `get_impact_graph` |
| `expensive` | Deep graph analysis | `analyze_code_relationships`, `pr_review`, `find_dead_code` |
| `background` | Long-running index/sync | `vector_index_repository`, `project_sync`, `watch_directory` |

Rules:

- Call `recommend_tools` when unsure which tool fits.
- Do not `resources/read` the full `codecortex://tools/catalog` on every turn; use it for integration setup or when `recommend_tools` is insufficient.
- Prefer one bounded tool over several expensive ones.

Compact category map: [references/mcp-tool-matrix.md](references/mcp-tool-matrix.md). Live per-tool cards: `codecortex://tools/catalog` or `cortex mcp tools --metadata`.

## Context-aware workflow

1. **Preflight** — `check_health`, `index_status`; repair index if unhealthy or stale.
2. **Classify intent** — relationship, impact, quality, refactor, context, patch, delta, diagnostics.
3. **Apply scope** — filters before expensive calls; start narrow.
4. **Run tool chain** — router below; cross-check once for high-impact answers.
5. **Respond** — evidence-first output (see Output contract).

## Context-aware router

| If user asks for | Primary tool | Secondary tool | Decision note |
| --- | --- | --- | --- |
| callers or callees | `analyze_code_relationships` | `get_impact_graph` | `find_callers` or `find_callees` |
| blast radius or impact | `get_impact_graph` | `analyze_code_relationships` | depth-bounded graph |
| path from A to B | `search_logic_flow` | `analyze_code_relationships` | `call_chain` for one direct path |
| dead code or complexity | `find_dead_code`, `calculate_cyclomatic_complexity` | `analyze_code_relationships` | strict filters first |
| patterns or architecture role | `find_patterns` | `find_code` | confidence threshold |
| refactor suggestions for symbol | `analyze_refactoring` | complexity + dead code | combine hotspots |
| symbol definition | `go_to_definition` | `get_signature` | graph index required |
| all usages | `find_all_usages` | `analyze_code_relationships` | graph index required |
| hover-style summary | `quick_info` | `get_signature` | cheap navigation |
| natural-language discovery | `vector_search_hybrid` | `get_context_capsule` | needs graph + vector |
| semantic search (vector only) | `vector_search` | `find_code` | vector index required |
| context for AI reasoning | `get_context_capsule` | `get_signature`, `get_skeleton` | bounded tokens |
| pre-edit patch planning | `get_patch_context` | `get_api_contract`, `get_test_context` | before broad file reads |
| multi-step review / consensus (kernel, PR, deadlock) | `cortex_a2a_spawn_session` | `manage_codecortex` (`spawn_a2a_session`) | requires `[a2a].enabled` in config.toml; one host call |
| changed branch context | `get_delta_context` | `branch_structural_diff`, `get_impact_graph` | review workflows |
| PR-style review | `pr_review` | `get_delta_context` | diff-aware when available |
| affected tests | `get_test_context` | `find_tests` | before editing tests |
| API surface for symbol | `get_api_contract` | `get_signature` | contracts before edits |
| module overview | `summarize_module` | `get_skeleton` | folder orientation |
| cost preview | `estimate_context_cost` | `get_patch_context` | before large context |
| cross-repo similarity | `find_similar_across_projects` | `vector_search_hybrid` | graph + vector |
| shared deps across projects | `find_shared_dependencies` | `list_projects` | multi-project |
| API compare across projects | `compare_api_surface` | `find_code` | multi-project |
| health or trustworthiness | `check_health`, `index_status`, `diagnose` | `explain_index_freshness` | mandatory before critical guidance |
| unsupported custom query | `execute_cypher_query` | — | last resort only |

For patch planning, branch review, and freshness repair, prefer [codecortex-workflows](../codecortex-workflows/SKILL.md).

## Response fields (context tools)

Read these on every context or analysis response before claiming impact:

- `freshness` — if `stale`, `partial`, or `unknown`, avoid high-confidence impact claims; repair index first.
- `source_policy` — respect snippet exposure limits.
- `budget_tokens` / estimated tokens — stay within budget; widen scope only deliberately.
- `warnings` — surface to the user.
- `suggested_next_tools` — follow when continuing the same task.

## Filter policy

Fields: `include_paths`, `include_files`, `include_globs`, `exclude_paths`, `exclude_files`, `exclude_globs`.

CLI parity: `--folder` / `--dir` → `include_paths`; `--file` → `include_files`.

Semantics: includes OR-combined; excludes OR-combined; excludes win.

Default order: module path → language glob → exclude generated/vendor/fixtures.

## Minimal MCP payloads

Relationship:

```json
{
  "query_type": "find_callers",
  "target": "authenticate",
  "include_paths": ["src/auth"],
  "include_globs": ["**/*.rs"],
  "exclude_paths": ["src/auth/generated"]
}
```

Patch context:

```json
{
  "task": "add token refresh to auth client",
  "include_paths": ["src/auth"],
  "budget_tokens": 6000,
  "mode": "feature"
}
```

Delta context:

```json
{
  "source_branch": "feature/auth-refresh",
  "target_branch": "main",
  "budget_tokens": 6000
}
```

## CLI fallback

Broad scans only:

```bash
cortex analyze smells . --min-severity warning --include-path src --exclude-file src/generated.rs
cortex analyze refactoring . --min-severity warning --include-path src
```

## Language-aware notes

Treat smell and refactor output as extension-aware. Coverage: `rs`, `py`, `rb`, `js`, `jsx`, `ts`, `tsx`, `go`, `java`, `c`, `cc`, `cpp`, `h`, `hpp`, `cs`, `php`, `swift`, `kt`, `kts`, `m`, `mm`, `scala`.

## Output contract

```markdown
## Scope
- Target: symbol or module or path
- Filters: include and exclude summary

## Findings
- [High/Med/Low] fact with path or symbol or metric

## Next Actions
1. small testable change
2. validation step
```

## Guardrails

- Prefer MCP over CLI whenever possible.
- Avoid unfiltered repo-wide scans unless requested.
- Cross-check once; do not infer causality from one query.
- `execute_cypher_query` only as last resort.
- Treat stale/partial/unknown `freshness` as a hard warning.
- Prefer metadata, signatures, skeletons, and snippets over full files for private repos.

## Failure handling

1. Re-run `check_health` and `index_status`.
2. Narrow filters and retry.
3. Complementary tool from router.
4. CLI fallback only for broad smells or refactoring.
5. Index repair per codecortex-indexing if tier or freshness blocks progress.

## Progressive disclosure

- [references/mcp-tool-matrix.md](references/mcp-tool-matrix.md)
- [references/tool-chains.md](references/tool-chains.md)
- [references/testing.md](references/testing.md)
