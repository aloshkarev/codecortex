# FalkorDB indexing performance analysis

**Date:** 2026-05-28  
**Fixture:** `crates/cortex-parser/tests/fixtures/sample_project_rust` (1 indexed file, ~9 edges, ~4 symbols)  
**Raw matrix:** [2026-05-29-falkordb-index-perf-results.md](2026-05-29-falkordb-index-perf-results.md)

## Executive summary

On the small Rust fixture, **indexing is parse-bound**, not graph-bound (`phase_edge_flush_secs` ≈ 3–4 ms vs `phase_parse_loop_wall_secs` ≈ 100 ms). FalkorDB graph writes are functional after fixing **CYPHER scalar quoting** for path-like parameters (repository paths with `/`).

For kernel-scale repos, expect **graph-bound** behavior; micro-benchmarks and payload tests show the main FalkorDB risks are **serial `GRAPH.QUERY`**, **inline UNWIND literal size** (especially `source`), and **per-relationship-type round trips**.

## Phase breakdown (FalkorDB, `CORTEX_INDEX_PROFILE=1`)

| case | duration_s | phase_edge_flush_s | edges | edges/s | bolt_exec | max_query_B | lock_wait_frac |
|------|------------|-------------------|-------|---------|-----------|-------------|----------------|
| baseline (2048, 256K source cap) | 0.140 | 0.004 | 9 | 2238 | 3 | 2803 | 0.003 |
| batch 1024 | 0.139 | 0.004 | 9 | 2349 | 3 | 2803 | 0.003 |
| batch 4096 | 0.139 | 0.004 | 9 | 2301 | 3 | 2803 | 0.003 |
| source_cap 0 | 0.143 | 0.004 | 9 | 2464 | 3 | 2803 | 0.005 |
| source_cap 32K | 0.138 | 0.004 | 9 | 2450 | 3 | 2803 | 0.004 |

**Decision gate (tiny repo):** parse > 50% → parser/parallelism first; graph tuning on this fixture is noise.

## Bulk throughput micro-benchmark (`falkordb_bulk_throughput_test`)

| Workload | Throughput | Notes |
|----------|------------|--------|
| 2000 nodes (no source) | ~4313 nodes/s | 15 `GRAPH.QUERY` calls, max ~2.8 KB |
| 1999 edges (one rel type in batch) | ~15040 edges/s | 1 bolt execution (single CALLS group) |
| 500 nodes × 1 KiB `source` | ~2200 nodes/s | **max inline query ~656 KB** |

## Hypothesis verdicts

| ID | Hypothesis | Verdict | Evidence |
|----|------------|---------|----------|
| **H1** | Serial writes cap throughput | **Confirmed for large repos** | Single `Mutex` on `MultiplexedConnection`; lock_wait_frac low on tiny repo but wall time stacks with query count (15 queries / run) |
| **H2** | Huge inline queries dominate | **Confirmed** | `build_node_batch_param` embeds `source`; 500 × 1 KiB → 656 KB max query; default 256 KiB cap bounds per-node source before inline |
| **H3** | Suboptimal batch size | **Inconclusive on tiny fixture** | 1024–4096 unchanged; sweep on medium/kernel repo required |
| **H4** | Edge MATCH without indexes | **Partial** | `ensure_constraints` runs; `CALL db.indexes()` unavailable on test image; EXPLAIN on edge MERGE succeeds |
| **H5** | Rel-type splitting inflates RTTs | **Confirmed** | 9 edges → 3 `bolt_executions` (CALLS, CONTAINS, HAS_PARAMETER) |
| **H6** | Resolve passes costly | **Not on tiny fixture** | resolve phases < 2 ms total |
| **H7** | Typed params faster than inline | **Open** | Official `falkordb` crate still blocked by rustc; needs FalkorDB-native map param API spike |
| **H8** | Server resource bound | **Open** | Not measured on this host; co-locate indexer + FalkorDB for production sweeps |

## Bugs found during analysis

1. **CYPHER scalar paths must be quoted** — `CYPHER repo=/path` fails parsing; fixed in `falkordb_params` (`'...'` wrapping for string prefix values).
2. **Perf scripts must isolate `HOME`** — `CortexConfig` loads `~/.cortex/config.toml` only; matrix script uses `HOME=$WORK_HOME` for index runs.

## Instrumentation added

- `CORTEX_FALKORDB_PROFILE=1` (also enabled with `CORTEX_INDEX_PROFILE=1`)
- `FalkorDbProfileSnapshot` on `IndexReport.falkordb_profile` (query count/bytes, lock wait, wall time)
- `target: cortex_graph::falkordb_profile` debug logs per `GRAPH.QUERY`
- Script: `scripts/falkordb-index-perf-matrix.sh`
- Tests: `falkordb_bulk_throughput_test`, `falkordb_index_explain_test`

## Recommended config (starting point)

```toml
backend_type = "falkordb"
memgraph_uri = "falkor://127.0.0.1:6379"
falkordb_graph = "codecortex"
max_batch_size = 2048
memgraph_unwind_batch_max = 2048
graph_node_source_max_bytes = 32768   # lower if graph writes dominate; 0 = smallest queries
memgraph_write_pool_size = 2        # FalkorDB: N parallel GRAPH.QUERY connections (shard bulk UNWIND)
```

Profile a real run:

```bash
RUST_LOG=error CORTEX_INDEX_PROFILE=1 CORTEX_FALKORDB_PROFILE=1 \
  cortex index /path/to/repo --force --format json | jq '.falkordb_profile'
```

## Implementation backlog (priority)

| Priority | Item | Expected impact |
|----------|------|-----------------|
| **P0** | Done: quote CYPHER string scalars | Correctness for all repos with path params |
| **P1** | FalkorDB **write connection pool** — implemented via `memgraph_write_pool_size`; tune 2–4 on multi-core when graph-bound | 2–4× graph phase when lock_wait_fraction high |
| **P1** | **Slim node UNWIND** — omit `source` from bulk MERGE or optional `index_graph_source=false` | Large reduction in inline query bytes |
| **P2** | Sweep `max_batch_size` / `graph_node_source_max_bytes` on **medium + kernel** fixtures | Find Falkor-specific optimum (may be < Memgraph) |
| **P2** | FalkorDB index verification (`db.indexes` or docs) + `warn_if_falkordb_codenode_id_index_missing` | Faster edge MATCH at scale |
| **P3** | Native / JSON batch params (avoid full literal inline) | Lower client CPU + network bytes |
| **P3** | Edge rel-type fusion (only if `bolt_multiplier` high on large repos) | Fewer round trips |

## Next measurements

1. Re-run matrix on a **5–20k file** OSS repo with `RUST_LOG=error` and compare FalkorDB vs Memgraph (`memgraph_write_pool_size = 4`).
2. Kernel tree index with profiling; confirm `phase_edge_flush_secs / duration_secs > 0.5`.
3. Record `falkordb_profile.query_bytes_max` and `lock_wait_fraction` per run in CI nightly (optional).
