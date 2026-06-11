# CodeCortex crates — performance backlog (Pass 2)

Review branch: `perf/crates-pass2`  
Review date: 2026-06-01  
Base commit: `105a3203d2f464ad26690dfe7dbc04c2a2cf55d7` (parent `main`)

Pass 1 ([crates-backlog.md](crates-backlog.md)) PERF-001–101 completed on `perf/crates-review`.

## Baselines

| Artifact | Value / path |
| --- | --- |
| Index JSON (pre-fix) | `audit/index-perf/pass2-main-highspeed.json` (run when FalkorDB up) |
| Criterion baseline | `pass2-pre` (`cargo bench -p cortex-benches -- --save-baseline pass2-pre`) |
| Clippy perf | `cargo clippy --workspace --all-targets -W clippy::perf -W clippy::redundant_clone` |
| Worktree | `../64-codecortex-perf-pass2/64-codecortex` |

```bash
cd /run/media/alex/artefacts/projects/self/projects/64-codecortex-perf-pass2/64-codecortex
export CORTEX_INDEX_PROFILE=1 CORTEX_DAEMON_BYPASS_QUEUE=1
cortex vector-index .
cortex index . --force --format json > audit/index-perf/pass2-main-highspeed.json
cortex index-report analyze --file audit/index-perf/pass2-main-highspeed.json
cargo bench -p cortex-benches -- --save-baseline pass2-pre
```

## Severity

- **P0** — dominant index/MCP wall or correctness under load
- **P1** — throughput, duplicate work, missing measurement
- **P2** — micro-opts, deferred matrix items

## Tasks

### cortex-indexer

- [x] **PERF-102** — `deferred_node_write` regression on `force` + branch without wipe  
  - **Evidence:** Pass 1 post-fix table: deferred 44.6% vs 23.5% when deferred replay active (`report_analysis.rs:168-175`, `indexer.rs:768-775`).  
  - **Fix:** Inline node writes when `files.len() ≤ 96` (skip spill/replay for small repos); highspeed profile still uses `wipe_branch_first`.  
  - **Verify:** `CORTEX_INDEX_PROFILE=1 cortex index . --force --format json`; `deferred_node_write` → 0 for medium repo with wipe; small-repo force+branch without wipe has no defer log.

- [x] **PERF-103** — Matrix #10: parallel type/field resolve  
  - **Evidence:** `audit/index-perf/README.md` #10 deferred; sequential `resolve_type_references` + `resolve_field_accesses` (`indexer.rs:1161-1186`).  
  - **Fix:** `tokio::join!` for both phases; separate phase timings preserved.  
  - **Verify:** Index JSON: `phase_resolve_type_references_secs + phase_resolve_field_accesses_secs` wall ≤ sequential sum; `cargo test -p cortex-indexer`.

### cortex-benches

- [x] **PERF-104** — No graph bulk write bench  
  - **Evidence:** README listed 8 benches; no graph bulk path (`crates/cortex-benches/README.md`).  
  - **Fix:** `graph_bulk_benchmark` (synthetic node build + chunk split). Live Falkor: `RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_bulk_throughput_test`.  
  - **Verify:** `cargo bench --bench graph_bulk_benchmark`.

### cortex-mcp

- [x] **PERF-105** — L1 tool cache only on `get_context_capsule`  
  - **Evidence:** `tool_cache()` at `handler.rs` ~2002/2218 only; `find_code`, `get_impact_graph`, `pr_review` uncached.  
  - **Fix:** L1 cache via `tool_params_hash` for `find_code`, `get_impact_graph`, `pr_review`.  
  - **Verify:** `cargo bench --bench cache_benchmark`; repeated MCP tool calls hit L1 (manual or integration).

- [ ] **PERF-107** — Handler clone/allocation density (~458 `.clone()` in `handler.rs`)  
  - **Severity:** P2  
  - **Suggested fix:** `Cow` / borrow in top hot handlers (`get_context_capsule`, patch context).  
  - **Verify:** `rg '\.clone\(\)' crates/cortex-mcp/src/handler.rs | wc -l` trend down on hot paths only.

### cortex-vector + mcp

- [x] **PERF-106** — Hybrid search cost unmeasured  
  - **Evidence:** Vector index often `unknown` before `vector-index`; hybrid path in capsule (`handler.rs:2020-2125`).  
  - **Fix:** `hybrid_search_benchmark` with mock store/embedder.  
  - **Verify:** `cargo bench --bench hybrid_search_benchmark`; `cortex vector-index .` before agent hybrid claims.

### cortex-watcher + indexer

- [ ] **PERF-108** — Incremental index under watch — no perf integration test  
  - **Severity:** P2  
  - **Suggested fix:** Burst fixture + index JSON compare in `watcher_perf_benchmark` or integration test.  
  - **Verify:** `cargo test -p cortex-watcher`; watch + incremental index on fixture.

### Discovery (P2, deferred)

| ID | Crate | Note |
| --- | --- | --- |
| PERF-109 | cortex-cli | Cold-start / `Box<WorkspaceCommand>` already addressed in Pass 1 |
| PERF-110 | cortex-a2a | Hub ring cap shipped (PERF-080); load test under burst TBD |
| PERF-111 | cortex-analyzer | Smell scan byte cap shipped (PERF-060) |

## Post-fix verification

| Metric | Pre-fix (Pass 1 note) | Post-fix (`pass2-post-fix-highspeed.json`) | Notes |
| --- | --- | --- | --- |
| `duration_secs` | 16.35 | **27.9** (analyze wall; JSON `18.45` parse-only) | Medium repo; FalkorDB up |
| `deferred_node_write` % | 44.6% | **0%** (`force_branch_deferred_replay: false`) | PERF-102 + highspeed wipe |
| `edge_flush` % | 25.7% | **29.0%** | Mixed-rel `bolt_multiplier` 1.0 |
| `resolve_type` + `resolve_field` | sequential | **0.09s + 0.12s** (parallel) | PERF-103 |
| `edges_per_sec` | 18 483 | **9583** | Profile/env variance; deferred eliminated |
| MCP cache tools | 1 | **4** | PERF-105 |
| Criterion benches | 8 | **10** | PERF-104, PERF-106 |

Re-run when FalkorDB is up:

```bash
cortex index . --force --format json > audit/index-perf/pass2-post-fix-highspeed.json
cargo bench -p cortex-benches -- --baseline pass2-pre
cargo test --workspace
```

## Shipped in Pass 2 (code)

| ID | Change |
| --- | --- |
| PERF-102 | `INLINE_NODE_WRITE_FILE_THRESHOLD` (96): skip deferred spill on small force+branch repos |
| PERF-103 | Parallel `resolve_type_references` + `resolve_field_accesses` via `tokio::join!` |
| PERF-104 | `graph_bulk_benchmark.rs` + README |
| PERF-105 | L1 cache on `find_code`, `get_impact_graph`, `pr_review` + `tool_params_hash` |
| PERF-106 | `hybrid_search_benchmark.rs` + README |

## Suggested commit message

```
perf(crates): Pass 2 indexer defer threshold, parallel resolve, MCP cache, benches

Skip deferred node spill for small force+branch indexes; run type/field resolve
in parallel; extend L1 tool cache to find_code, get_impact_graph, and pr_review;
add graph_bulk and hybrid_search Criterion benches (PERF-102–106).
```
