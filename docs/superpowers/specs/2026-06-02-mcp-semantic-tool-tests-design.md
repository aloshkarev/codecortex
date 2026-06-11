# MCP semantic tool tests — design spec

**Status:** Approved for implementation (2026-06-02)  
**Repo:** 64-codecortex (self-repo ground truth)  
**Related:** [MCP_AUDIT_FINDINGS.md](../../MCP_AUDIT_FINDINGS.md), plan `mcp_semantic_tool_tests`

## Goals

1. Detect tools that return empty or `NOT_FOUND` when the graph and source tree contain the anchor symbol.
2. Align tool behavior with `#[tool(description)]` / `tool_cards()` claims.
3. PR gate on ~20 high-value graph/navigation tools; nightly coverage for all 77 exported tools.

## Non-goals

- Replace envelope/schema contract tests in `contract_tests.rs` (extended instead: `success_json` + migrated-tool list).
- Block PR on full-repo vector quality (requires Ollama); PR uses `vector_pr` fixture + `HashEmbedder` instead.
- Block PR on A2A, destructive, or long-running index tools (PR smoke runs scoped/dry-run variants; A2A uses `--a2a-chain` in nightly).

## Envelope contract

All MCP tool responses MUST use the envelope contract (`status`, `meta`, `warnings`, `data`); legacy bare JSON is no longer allowed in handlers.

### Nightly A2A semantic chain

Oracles with `"skip": "a2a_chain"` run when `mcp_semantic_audit.py --a2a-chain` is passed (wired in `scripts/nightly-mcp-audit.sh`): spawn session once, inject `task_id` into get/send/subscribe/cancel/list_push oracles.

## Oracle registry

**Path:** [`tests/mcp_semantic/oracles.json`](../../../tests/mcp_semantic/oracles.json)

| Field | Required | Meaning |
| --- | --- | --- |
| `tool` | yes | MCP tool name |
| `profile` | yes | `pr` and/or `nightly` |
| `tier` | yes | `none`, `graph`, `vector`, `graph_and_vector` |
| `description_claim` | yes | Short behavioral claim (L1 drift check) |
| `description_keywords` | no | Substrings that must appear in tool description |
| `args` | yes | `tools/call` arguments; `${REPO}` and `${FIXTURE}` expanded at runtime |
| `assertions` | yes | List of checks on parsed JSON body |
| `negative_control` | no | Second call that must not false-positive |
| `allow_status` | no | e.g. `partial` when empty hits are acceptable |

### Assertion types

| Type | Fields | Semantics |
| --- | --- | --- |
| `status_in` | `values` | Top-level envelope `status` in set |
| `not_error` | — | `status` not `error` |
| `min_length` | `path`, `min` | Array at JSON pointer length ≥ min |
| `max_length` | `path`, `max` | Array length ≤ max |
| `contains` | `path`, `substring` | String field contains substring |
| `exists` | `path` | Value not null/missing |
| `anchor_absent` | `path`, `field`, `value` | No array element has `field == value` (dead-code guard) |
| `one_of_tools` | `path`, `tools` | Any suggested tool name in list appears in array at path |
| `rank_contains` | `path`, `field`, `substring`, `max_rank` | Result at rank contains substring in field (default rank 0) |
| `gte` | `path`, `min` | Numeric field ≥ min |
| `scores_descending` | `path`, `field` | Scores in array non-increasing |
| `negative_rank_absent` | `path`, `field`, `substring`, `top_k` | Substring must not appear in top-k hits |

JSON pointers use slash form: `data/definitions` (no `$` prefix).

## Profiles

### PR (`profile` includes `pr`)

Runs in CI on indexed 64-codecortex: preflight, routing, navigation, relationships, quality (scoped), agent context, branch diff, patterns, diagnose.

**Preflight:** `CORTEX_SEMANTIC_REPO` must have `graph` freshness `fresh` (setup job runs `cortex index` if needed).

**Timeout budget:** &lt; 3 minutes total for PR semantic pass.

### Vector PR (`profile` includes `vector_pr`)

Runs on [`tests/fixtures/vector_semantic/`](../../../tests/fixtures/vector_semantic/) with `CORTEX_TEST_EMBEDDER=1` (deterministic `HashEmbedder`). Bootstrap: `make mcp-vector-semantic-pr`.

Five tools: `vector_index_repository`, `vector_index_status`, `vector_search`, `vector_search_hybrid`, `vector_index_file`. Assertions include ranked path anchors (`rank_contains`) and hybrid score ordering.

### Nightly (`profile` includes `nightly`)

All PR oracles plus: vector trio, index/watch/admin, A2A, multi-project, cypher, bundle/memory, destructive dry-run smoke oracles.

**Preflight:** `cortex vector-index` on repo before vector oracles.

## Harnesses

| Harness | Role |
| --- | --- |
| [`scripts/mcp_semantic_audit.py`](../../../scripts/mcp_semantic_audit.py) | Primary CI/Makefile driver; writes `target/mcp-semantic-ledger.json` |
| [`crates/cortex-mcp/tests/mcp_semantic_audit.rs`](../../../crates/cortex-mcp/tests/mcp_semantic_audit.rs) | Rust integration (`CORTEX_SEMANTIC_AUDIT=1`) |
| [`crates/cortex-mcp/tests/mcp_vector_semantic.rs`](../../../crates/cortex-mcp/tests/mcp_vector_semantic.rs) | Rust vector fixture gate (`CORTEX_VECTOR_SEMANTIC=1`) |
| [`scripts/mcp_tool_audit.py`](../../../scripts/mcp_tool_audit.py) | Smoke; semantic script sets stricter empty rules when oracle present |

## Environment variables

| Variable | Default | Purpose |
| --- | --- | --- |
| `CORTEX_SEMANTIC_REPO` | workspace root | Indexed repository path |
| `CORTEX_SEMANTIC_PROFILE` | `pr` | `pr`, `nightly`, or `vector_pr` |
| `CORTEX_SEMANTIC_FIXTURE` | `tests/fixtures/vector_semantic` | Fixture path for `vector_pr` |
| `CORTEX_TEST_EMBEDDER` | — | Set `1` for deterministic `HashEmbedder` (CI vector gate) |
| `CORTEX_BIN` | `cortex` | CLI for `mcp start` |
| `CORTEX_SEMANTIC_AUDIT` | — | Set `1` to run Rust ignored test |
| `CORTEX_VECTOR_SEMANTIC` | — | Set `1` to run Rust vector fixture test |
| `CORTEX_LIVE_MCP_AUDIT` | — | Optional combined smoke + semantic in cli test |

## Failure taxonomy

| Code | Meaning | Action |
| --- | --- | --- |
| `EMPTY_WHEN_GRAPH_HAS` | `min_length` failed on anchor symbol | Fix resolver/navigation or index |
| `DESCRIPTION_DRIFT` | Keywords not in handler description | Update docs or oracle |
| `FRESHNESS_BLOCK` | Graph not fresh | `cortex index --force` |
| `VECTOR_NOT_READY` | Vector oracle with zero hits | `cortex vector-index` (nightly) |
| `RANK_MISS` | `rank_contains` failed on semantic query | Fix embedder/index or oracle anchor |
| `NEGATIVE_HIT` | Bogus query returned anchor in top-k | Fix ranking or relax negative oracle |
| `ENVELOPE_ERROR` | `status: error` | Handler/args bug |

## CI

New workflow job `mcp-semantic-pr` in [`.github/workflows/mcp-semantic-audit.yml`](../../../.github/workflows/mcp-semantic-audit.yml): FalkorDB service, build CLI, index repo, `make mcp-semantic-audit PROFILE=pr`.

Nightly: [`.github/workflows/mcp-nightly-audit.yml`](../../../.github/workflows/mcp-nightly-audit.yml) (matrix: `test` embedder plumbing + `ollama` real semantics) and [`scripts/nightly-mcp-audit.sh`](../../../scripts/nightly-mcp-audit.sh).

## Anchors

| Symbol | File |
| --- | --- |
| `resolve_project_context` | `crates/cortex-mcp/src/tools.rs` |
| `build_symbol_resolver` | `crates/cortex-analyzer/src/symbol_resolve.rs` |
| `__codecortex_no_such_symbol__` | negative control |
