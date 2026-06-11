# FalkorDB backend

CodeCortex uses **FalkorDB** as the graph backend for indexing. The driver speaks Redis RESP (`GRAPH.QUERY`), not Bolt.

## Quick start (Docker)

```bash
docker run -d --name codecortex-falkordb -p 6379:6379 falkordb/falkordb:latest
```

`~/.cortex/config.toml`:

```toml
backend_type = "falkordb"
falkordb_uri = "falkor://127.0.0.1:6379"
falkordb_graph = "codecortex"
# Defaults are highspeed (see Performance tuning). On laptops:
# indexing_profile = "conservative"
```

With Redis `requirepass`:

```toml
falkordb_password = "your_redis_password"
```

Verify:

```bash
cortex doctor
cortex index /path/to/repo --force
```

## Configuration

| Field | Default | Purpose |
|-------|---------|---------|
| `backend_type` | `falkordb` | Select FalkorDB driver |
| `falkordb_uri` | `falkor://127.0.0.1:6379` | Connection URL (`falkor://`, `redis://`, `rediss://`) |
| `falkordb_password` | `""` | Redis AUTH when set |
| `falkordb_graph` | `codecortex` | Graph name for `GRAPH.QUERY` |
| `indexing_profile` | `highspeed` | `conservative` restores smaller batches and single-threaded parse (low RAM) |
| `daemon_index_workers` | `min(4, cpus/2)` | Concurrent daemon index jobs across **different** repos |
| `falkordb_write_pool_size` | host-aware (2–8) | Parallel `GRAPH.QUERY` connections for bulk node/edge UNWIND (shard by id / `from`) |
| `falkordb_bulk_index_include_source` | `false` | When false, bulk node upserts omit `source`, `docstring`, and JSON `properties` for smaller queries |

Repository scoping uses the `repository_path` property on nodes (single graph per CodeCortex instance).

## Bulk indexing

Indexer throughput uses `UNWIND` batches. FalkorDB's Rust client does not accept Bolt-style typed map parameters reliably, so CodeCortex **inlines list-of-maps literals** for `$batch` / `$paths` / `$ids` while scalar parameters use `CYPHER key=value` prefixes.

## Sizing

FalkorDB runs out-of-process; the indexer does not load the full graph into its own address space. Size the FalkorDB container/host for your repository graph (node/edge counts), not the full source tree size.

## Performance tuning

### Profiling

```bash
RUST_LOG=error CORTEX_INDEX_PROFILE=1 CORTEX_FALKORDB_PROFILE=1 \
  cortex index /path/to/repo --force --format json > report.json
jq '.phase_edge_flush_secs, .duration_secs, .falkordb_profile' report.json
cortex index-report analyze --file report.json
```

`falkordb_profile` (when profiling is on) includes:

- `query_count`, `query_bytes_max`, `query_bytes_avg`
- `lock_wait_fraction` — time waiting on per-connection mutexes (raise `falkordb_write_pool_size` when high)
- `query_wall_secs` — server round-trip time

### Knobs

| Knob | Effect |
|------|--------|
| `indexing_profile` | `highspeed` (default): batch 4096, parse pipeline 1, all-core Rayon, parallel resolve chunks. `conservative` for &lt;8 GiB RAM |
| `max_batch_size` / `falkordb_unwind_batch_max` | Rows per inlined `UNWIND` batch (4096 highspeed, 2048 conservative) |
| `graph_node_source_max_bytes` | Truncate per-node `source` when `falkordb_bulk_index_include_source = true` (64KiB highspeed) |
| `falkordb_write_pool_size` | `min(cpus/2, 8)` highspeed; `2` conservative — raise when `lock_wait_fraction` is high |
| `falkordb_bulk_index_include_source` | `true` only if MCP/tools need full `source` on graph nodes during indexing |
| `indexer_parse_batch_size` | Files per Rayon parse batch (`256` highspeed, `160` conservative) |
| `indexer_parse_threads` | `Some(0)` = global Rayon pool (highspeed); `None` = CPUs − 1 (conservative) |
| `indexer_parse_pipeline_depth` | `1` overlaps next parse batch with graph writes (highspeed) |

One-off conservative run: `cortex index /path --profile conservative` or `CORTEX_INDEX_PROFILE=conservative`.

Validation matrix: [audit/index-perf/README.md](../../audit/index-perf/README.md).

### Force + git branch: deferred node replay

`cortex index --force` on a **git branch** defers graph node writes until after parse (so a timeout does not delete the old branch graph early). That **deferred_node_write** phase replays spilled nodes to FalkorDB and often dominates wall time on kernel-scale trees.

| Approach | Effect |
|----------|--------|
| **Incremental** (`cortex index` without `--force`, or `--mode incremental-diff`) | No deferred replay; nodes written during parse |
| **`cortex index --wipe-branch-first`** | Deletes branch graph **before** parse; inline node writes; **no** deferred phase (timeout mid-run may leave an empty branch) |
| TOML `index_force_delete_branch_before_parse = true` | Same as `--wipe-branch-first` |
| `index_include_files` / excludes | Smaller deferred spill |

Index reports include `deferred_spill_read_secs`, `deferred_collect_secs`, `deferred_write_nodes_secs`, and `deferred_spill_bytes`. Use `cortex index-report analyze` when deferred fraction is high.

Avoid routine `cortex index --force` on a tracked branch when incremental-diff is enough. Prefer incremental-diff for faster re-index.

### Matrix script

```bash
RUN_DOCKER_INTEGRATION=1 ./scripts/falkordb-index-perf-matrix.sh /path/to/repo
```

Results: `docs/superpowers/specs/2026-05-29-falkordb-index-perf-results.md`  
Analysis: `docs/superpowers/specs/2026-05-29-falkordb-index-perf-analysis.md`

## Integration tests

```bash
RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_bulk_upsert_smoke -- --ignored
RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_bulk_throughput_test -- --ignored --nocapture
RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_index_explain_test -- --ignored
```
