# Index performance validation matrix

Evidence for default **highspeed** indexing profile and parallel `resolve_call_targets`.

## How to reproduce

```bash
export CORTEX_DAEMON_BYPASS_QUEUE=1
export CORTEX_INDEX_PROFILE=1
export CORTEX_FALKORDB_PROFILE=1

# conservative (old-style defaults)
CORTEX_INDEX_PROFILE=conservative cortex index . --force --format json > audit/index-perf/medium-conservative.json

# highspeed (code defaults)
cortex index . --force --format json > audit/index-perf/medium-highspeed.json

cortex index-report analyze --file audit/index-perf/medium-highspeed.json
```

Matrix script (Docker FalkorDB):

```bash
RUN_DOCKER_INTEGRATION=1 ./scripts/falkordb-index-perf-matrix.sh [REPO_PATH]
```

## Solution validation matrix

| # | Solution | Status | Notes |
| --- | --- | --- | --- |
| 1 | Rayon parse batches + thread-local tree-sitter | Shipped | `parse_file_batch_timed`, `parse_pool.rs` |
| 2 | `indexer_parse_pipeline_depth = 1` | Shipped | Overlap parse N+1 with graph writes for N |
| 3 | `indexer_parse_threads = Some(0)` | Shipped | Global Rayon pool |
| 4 | `indexer_parse_batch_size = 256` | Shipped | Default in highspeed profile |
| 5 | `memgraph_write_pool_size` 2–8 | Shipped | `default_write_pool_size()` |
| 6 | `max_batch_size` / unwind 4096 | Shipped | Highspeed default |
| 7 | `falkordb_bulk_index_include_source = false` | Shipped | Unchanged |
| 8 | `graph_node_source_max_bytes` 64KiB | Shipped | Highspeed default |
| 9 | Chunked parallel `resolve_call_targets` | Shipped | 256-id UNWIND chunks, pool-sharded |
| 10 | Parallel type/field resolve | Deferred | Only if phases >10% after #9 |
| 11–12 | Incremental / watcher diff | Shipped | Operational |
| 13 | Multi-worker daemon | Shipped | `daemon_index_workers` |
| 14 | One index per repo | Shipped | Unchanged |

## Pass gates (medium repo: 64-codecortex)

| Gate | Target | Baseline (pre-change) |
| --- | --- | --- |
| G1 pipeline | Parse overlap ↓ wall or total ↓ | — |
| G2 pool | `lock_wait_fraction` ≤ 0.20 | — |
| G3 batch | No OOM; duration within 5% or ↓ | — |
| G4 resolve | `phase_resolve_call_targets_secs / duration_secs` < 0.35 | ~0.81 (~59s / ~73s) |
| G5 e2e | `duration_secs` ≥ 30% ↓ vs conservative | — |

Re-run after FalkorDB is up and commit JSON under `audit/index-perf/{small,medium,large}-*.json`.

## Profiles

| Profile | When | CLI |
| --- | --- | --- |
| `highspeed` | Default (≥8 GiB dev hosts) | `cortex index …` |
| `conservative` | Laptops / low RAM | `cortex index --profile conservative` or `CORTEX_INDEX_PROFILE=conservative` |

TOML: `indexing_profile = "conservative"` in `~/.cortex/config.toml`.
