# Context packs

## Schema

Full JSON schema: `resources/read` → `codecortex://schema/context-pack`.

Context tools (`get_patch_context`, `get_delta_context`, `get_context_capsule`, `get_test_context`) return bounded packs aligned with that schema.

## Common fields

| Field | Use |
| --- | --- |
| `meta.freshness` | Trust gate before impact claims |
| `meta.source_policy` | Snippet exposure rules |
| `budget_tokens` / estimated tokens | Stay within budget |
| `warnings` | Show user |
| `suggested_next_tools` | Continue workflow |
| Targets, contracts, tests, risks | Edit and review planning |

## Modes (`get_patch_context`)

Set `mode` to match task shape (e.g. `feature` for additive work). When unsure, omit `mode` or follow `recommend_tools` output.

## Budgeting

1. Call `estimate_context_cost` before very large `budget_tokens`.
2. Start with 4000–8000 tokens for focused patches; increase only when packs omit critical symbols.
3. Prefer `get_api_contract` and `get_signature` over raising budget.

## After receiving a pack

1. Read `freshness` and `warnings` first.
2. Use contracts and test hints before opening files.
3. Follow `suggested_next_tools` for the next step in the same workflow.
4. Apply path filters on follow-up graph tools to match pack scope.

## Delta packs

`get_delta_context` needs `source_branch` and `target_branch` (and optional `budget_tokens`). Pair with `branch_structural_diff` for structural signals beyond file lists.
