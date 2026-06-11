# A2A ↔ MCP Cooperation Gap Memo (Phase 2 Batch 0)

**Date:** 2026-06-03  
**Scope:** Field-level parity between MCP context tools and A2A workflow intelligence after Phase 1 indexed-intelligence work.

## Method

Static code trace plus handler spawn path review. Live MCP session diff deferred to Batch 6 acceptance (`mcp_tool_audit.py --a2a-chain`).

## Field parity matrix

| Field | MCP `get_patch_context` | A2A `get_patch_context` / patch planner | Gap |
| --- | --- | --- | --- |
| `freshness` | `EnvelopeMeta.freshness` (often `Unknown`) | Not on capsule; spawn gets hub/handler freshness | **High** — A2A capsule lacks freshness |
| `warnings` | Envelope + omitted items | Not on capsule | **Med** |
| `suggested_next_tools` | `next_tools` on envelope | Spawn: handler sets 3 tools; hub returns empty | **Med** — hub path bypasses handler |
| `budget_tokens` / `estimated_tokens` | `TokenBudget` meta | Only inside patch JSON if caller inspects facade | **Med** |
| `source_policy` | `Snippets` on envelope | Absent | **Low** |
| Capsule `data` shape | `patch_context_json` | `PatchContextCapsule` summary only | **High** — A2A drops targets/contracts |

| Field | MCP `get_impact_graph` | A2A `analyze_impact` | Gap |
| --- | --- | --- | --- |
| Graph payload | Full JSON in `data` | `ImpactSummary` string only | **High** |
| `suggested_next_tools` | Present on envelope | Absent on blackboard | **Med** |
| `freshness` | On envelope | Absent | **Med** |

| Field | MCP `get_delta_context` | A2A `get_delta_context` | Gap |
| --- | --- | --- | --- |
| `freshness` | In delta JSON body | In delta JSON when graph works | **Low** |
| `suggested_next_tools` | Not always set | Absent | **Med** |

| Field | MCP `pr_review` | A2A `get_pr_review_summary` | Gap |
| --- | --- | --- | --- |
| Implementation | `ReviewAnalyzer` + graph in `handler.rs:1907` | Composes patch+impact+delta via facade | **High** — different code paths |
| Envelope meta | Full envelope | Plain `PrReviewSummary` struct | **High** |

| Field | MCP spawn | A2A hub spawn | Gap |
| --- | --- | --- | --- |
| `suggested_next_tools` | Set in `handler.rs:3759` | `hub.rs:338` empty until handler override | **Med** — CLI/direct hub use empty |
| Task artifacts | N/A | Partial capsule/summary JSON | **Med** — no `mcp_tool_id`, no pack meta |

## Structural gaps

1. **`a2a_facade.rs`** — Extra adapter layer; duplicates envelope assembly without MCP meta ([`a2a_facade.rs`](../../crates/cortex-mcp/src/a2a_facade.rs)).
2. **`session_scope` Mutex** — [`a2a_services.rs:18`](../../crates/cortex-mcp/src/a2a_services.rs); parallel `dispatch_parallel_and_record` can race.
3. **No `IntelligencePack`** — MCP uses `EnvelopeBuilder`; A2A uses per-tool structs ([`services.rs`](../../crates/cortex-a2a/src/services.rs)).
4. **No parity tests** — Docs claim parity ([A2A_COMPLETENESS.md](../../docs/A2A_COMPLETENESS.md)); no `intelligence_parity` test module.
5. **Semantic audit** — [`mcp_semantic_audit.py:367`](../../scripts/mcp_semantic_audit.py) verifies transport only, not artifact intelligence fields.
6. **Vector/hybrid** — Not used in A2A patch planner ([`patch.rs`](../../crates/cortex-mcp/src/intelligence/patch.rs) graph-only).
7. **External roles** — No `ToolDelegation` artifact with MCP tool hints when `agent_card_url` set.

## Recommended Phase 2 fixes (mapped to batches)

| Batch | Closes |
| --- | --- |
| 1 | IntelligencePack + shared meta on MCP tools |
| 2 | IntelligenceRequest; remove Mutex scope |
| 3 | tool_router; hub spawn initial tools; completion hints |
| 4 | pr_review/capsule/hybrid in intelligence; parity tests |
| 5 | External ToolDelegation artifacts |
| 6 | Oracles + audit scripts + docs |

## Exit criteria met

Field-level gap table with file references — **yes**.
