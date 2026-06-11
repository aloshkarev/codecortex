# MCP tools audit findings (64-codecortex)

Audit date: 2026-06-03 (live harnesses `scripts/mcp_tool_audit.py`, `scripts/mcp_semantic_audit.py`).

## Summary profiles

| Profile | BROKEN | DEGRADED | SKIPPED | Command |
| --- | --- | --- | --- | --- |
| **PR (default)** | 0 | 0 | 5 (A2A only) | `make mcp-tool-audit REPO=...` |
| **PR semantic (graph)** | 0 | — | — | `make mcp-semantic-pr REPO=...` |
| **PR semantic (vector)** | 0 | — | — | `CORTEX_TEST_EMBEDDER=1 make mcp-vector-semantic-pr` |
| **PR all gates** | 0 | 0 | 5 (A2A) | `make mcp-audit-all REPO=...` |
| **Nightly** | 0 target | 0 target | 0 (A2A via `--a2a-chain`) | `make mcp-nightly-audit` |

PR smoke now runs scoped long tools (`add_code_to_graph`, `vector_index_repository`, `find_dead_code`, `get_impact_graph`) with `wait` / `include_paths` / `max_files`, destructive tools with `dry_run: true`, and `remove_project` as post-audit cleanup. A2A task tools remain SKIPPED in default PR smoke; use `make mcp-tool-audit-a2a` or nightly `--a2a-chain`.

### Envelope migration (2026-06-03)

All legacy `Self::ok` handlers (~34 tools) now return the standard envelope (`status`, `meta`, `warnings`, `data`). L1 cache for `find_code` / `pr_review` stores envelope JSON via `success_json`. Cross-project vector tools flatten hits to match `vector_search` (`data/results[].metadata/path`).

## Production upgrades (SKIPPED / DEGRADED plan)

### Shared symbol resolution (W1)

- `crates/cortex-analyzer/src/symbol_resolve.rs` — repo-relative path normalization, definitional/callable kind matching, `SymbolResolver` for exact-then-fuzzy lookup.

### Navigation & signature (W2–W3)

- `go_to_definition` / `find_all_usages` — repo-relative `from_file`, optional `qualified_name`, envelope responses (`definitions` / `groups`, `partial`, suggestions).
- `get_signature` — resolver-backed exact match; `mcp.signature.enabled` (falls back to `mcp.skeleton.enabled`); `NOT_FOUND` with suggestions.

### Logic flow (W4)

- `search_logic_flow` — distinct `from_symbol` / `to_symbol`, parameterized Cypher with PascalCase kinds, structured `paths` with `nodes` / `edges`, `self_reference` for same-symbol queries.

### Long-running tools (W5)

- `add_code_to_graph` — optional `wait` + `wait_timeout_secs`.
- `find_dead_code` — `include_paths`, `limit`.
- `get_impact_graph` — `depth`, `budget_tokens` truncation.
- `vector_index_repository` — `include_paths`, `max_files`.

### Destructive safety (W6)

- `delete_repository` / `vector_delete_repository` — require `confirm: true` or `dry_run: true`.

### A2A (W7)

- `cortex_a2a_list_push_configs` — optional `task_id` lists all configs.
- `crates/cortex-mcp/tests/mcp_a2a_workflow.rs` — spawn → get_task → send_message → list_push (with/without task_id) → cancel.
- Audit `--a2a-chain` exercises full task chain.

## PR-mode SKIPPED inventory (5 — A2A only)

| Reason | Tools |
| --- | --- |
| A2A needs `task_id` (use `--a2a-chain` or `make mcp-tool-audit-a2a`) | `cortex_a2a_get_task`, `cortex_a2a_send_message`, `cortex_a2a_cancel_task`, `cortex_a2a_subscribe_task`, `cortex_a2a_list_push_configs` |

`remove_project` runs as post-audit cleanup (not counted as SKIPPED). Long/destructive tools run in PR with scoped args and `dry_run`.

## How to re-run

```bash
cargo build -p cortex-cli --release

# PR profile (fast; sets CORTEX_TEST_EMBEDDER for vector smoke)
make mcp-tool-audit REPO=/path/to/64-codecortex

# All PR gates (smoke + graph semantic + vector semantic)
make mcp-audit-all REPO=/path/to/64-codecortex

# PR + A2A chain
make mcp-tool-audit-a2a REPO=/path/to/64-codecortex

# Nightly 77/77 (fixture + long + dry-run destructive)
make mcp-nightly-audit

# Rust integration (graph + A2A)
CORTEX_TEST_GRAPH=1 cargo test -p cortex-mcp --test mcp_a2a_workflow -- --ignored

# CI gate (manual)
CORTEX_LIVE_MCP_AUDIT=1 cargo test -p cortex-cli --test mcp_live_tool_audit -- --ignored
```

Ledger: `target/mcp-audit-ledger.json`.

## Semantic audit (ground-truth oracles)

Registry: [`tests/mcp_semantic/oracles.json`](../tests/mcp_semantic/oracles.json). Spec: [`docs/superpowers/specs/2026-06-02-mcp-semantic-tool-tests-design.md`](superpowers/specs/2026-06-02-mcp-semantic-tool-tests-design.md).

| Profile | Tools | Command |
| --- | --- | --- |
| **PR (graph)** | 21 graph/navigation/context tools | `make mcp-semantic-pr REPO=...` |
| **PR (vector)** | 5 vector tools on fixture | `make mcp-vector-semantic-pr` |
| **Nightly** | 77 (incl. vector, A2A via `--a2a-chain`) | part of `make mcp-nightly-audit` |

```bash
make mcp-semantic-audit REPO=/path/to/64-codecortex PROFILE=pr
CORTEX_TEST_EMBEDDER=1 make mcp-vector-semantic-pr
# Ledger: target/mcp-semantic-ledger.json
# Failures: target/mcp-semantic-failures/<tool>.json
```

### Semantic failure taxonomy

| Code | Meaning |
| --- | --- |
| `EMPTY_WHEN_GRAPH_HAS` | Oracle `min_length` failed (e.g. `[]` but symbol exists in repo) |
| `DESCRIPTION_DRIFT` | `description_keywords` not in tool card (`cargo test -p cortex-mcp --test mcp_semantic_description`) |
| `FRESHNESS_BLOCK` | PR preflight: graph not `fresh` |
| `VECTOR_NOT_READY` | Nightly vector oracles after `cortex vector-index` |
| `RANK_MISS` | `rank_contains` failed (wrong top hit for NL query) |
| `NEGATIVE_HIT` | Negative control query ranked anchor file |
| `ENVELOPE_ERROR` | MCP `status: error` or RPC failure |

Smoke audit [`mcp_tool_audit.py`](../scripts/mcp_tool_audit.py) upgrades empty navigation/vector hits to **BROKEN** when a PR semantic oracle defines `min_length >= 1`.

### PR semantic baseline (2026-06-03)

| Result | Count |
| --- | --- |
| **VERIFIED** | **21 / 21** |
| **BROKEN** | 0 |

Ledger: `target/mcp-semantic-ledger.json`. CI semantic job is required (no `continue-on-error`).

### Vector PR semantic baseline (2026-06-03)

Fixture: [`tests/fixtures/vector_semantic/`](../tests/fixtures/vector_semantic/). Uses `HashEmbedder` via `CORTEX_TEST_EMBEDDER=1` (no Ollama/OpenAI on PR).

| Result | Count |
| --- | --- |
| **VERIFIED** | **5 / 5** |
| **BROKEN** | 0 |

```bash
CORTEX_TEST_GRAPH=1 CORTEX_TEST_EMBEDDER=1 make mcp-vector-semantic-pr
CORTEX_VECTOR_SEMANTIC=1 cargo test -p cortex-mcp --test mcp_vector_semantic -- --ignored
```

### `.cortexignore` (2026-06-03)

Shared `CortexIgnoreWalker` in `cortex-core/src/ignore.rs` drives graph index, vector index, and watch filtering. Global rules: `~/.cortex/cortexignore` or `global_cortexignore_path` in config. Fixture: `tests/fixtures/cortexignore/`.

#### Root-cause fixes (RC1–RC6)

| RC | Issue | Fix |
| --- | --- | --- |
| RC1 | Navigation kind filter UPPERCASE-only | `definitional_kind_cypher_predicate` (PascalCase + UPPERCASE) in `navigation.rs` |
| RC2 | Call-graph queried `:Function` | CodeNode-centric `CALLS` in `analyzer.rs` (+ CallTarget fallback) |
| RC3 | `get_signature` empty graph `source` | File-backed `load_source_snippet_for_signature`; partial envelope on parse miss |
| RC4 | `search_logic_flow` empty cross-symbol paths | Self-reference oracle (`tool_names`→`tool_names`); CodeNode flow Cypher unchanged |
| RC5 | Complexity on `:Function` | `CodeNode` + `coalesce(cyclomatic_complexity)`; direct `query_with_params` (no broken `raw_query_scoped` inject) |
| RC6 | Legacy bare arrays | `analyze_code_relationships` / `calculate_cyclomatic_complexity` use `envelope_success` + `data.results` |

Harness: `get_at` in `mcp_semantic_audit.py` supports numeric array indices (e.g. `data/definitions/0/file_path`).

## Vector repair (operator)

```bash
cortex doctor
cortex vector-index /path/to/64-codecortex
```
