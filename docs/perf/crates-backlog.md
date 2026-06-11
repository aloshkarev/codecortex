# CodeCortex crates — performance backlog

Review branch: `perf/crates-review`  
Review date: 2026-06-01

## Baselines

| Artifact | Value / path |
| --- | --- |
| Index JSON | `audit/index-perf/review-medium-highspeed-pure.json` |
| Index wall time | 15.45s (283 files, 77 565 edges) |
| `edges_per_sec` | 11 859 |
| Dominant phases | `edge_flush` 42.3%, `parse_loop_wall` 27.3%, `deferred_node_write` 23.5% |
| `resolve_call_targets` | 3.7% (gate G4 passed) |
| `bolt_multiplier` | 7.5 (rel-type splitting) |
| Criterion baseline | `pre-review` (see `cargo bench -p cortex-benches`) |
| Clippy perf sweep | No `clippy::perf` hits on workspace + path-dep crates |
| MCP graph freshness | `fresh` on `perf/crates-review` @ `b70d55ea` |

Index analyze flags: deferred replay (force+branch), rel-type splitting, FalkorDB max inline query 0.6 MiB.

## Severity

- **P0** — user-visible latency or dominant index/MCP wall time
- **P1** — throughput, memory, or duplicate work at scale
- **P2** — micro-opts, benchmarks, future work

## Tasks

### cortex-indexer

- [x] **PERF-001** — Edge spill double JSON encode/decode on flush  
  - **Evidence:** `edge_spill.rs:37-42` serialize per push; `98-109` deserialize per line.  
  - **Suggested fix:** Binary/columnar spill or in-memory chunk handoff to graph writer.  
  - **Verify:** `CORTEX_INDEX_PROFILE=1` index; compare `edge_spill_read_secs` vs `edge_spill_bolt_secs`.

- [x] **PERF-002** — Deferred node replay (~23.5% wall) on `force` + branch  
  - **Evidence:** Index analyze `force_branch_deferred_replay`; `indexer.rs:768-775`, `1046-1066`.  
  - **Suggested fix:** Prefer incremental index or `--wipe-branch-first` when full rebuild not needed.  
  - **Verify:** Re-index without `--force`; deferred phase → 0.

- [x] **PERF-003** — `DeferredFileRecord` clones full `nodes` per file  
  - **Evidence:** `edge_spill.rs:155-162`, replay `indexer.rs:1500-1501`.  
  - **Suggested fix:** Spill node ids only or inline writes with `wipe_branch_first`.  
  - **Verify:** RSS after_discover vs after_parse; `deferred_spill_bytes` in report.

- [x] **PERF-004** — Duplicate `CONTAINS` edges for shared directories  
  - **Evidence:** `spill_file_edges` ignored `seen_dirs.insert` return (`indexer.rs:1589-1616`).  
  - **Suggested fix:** Emit dir edges only when `seen_dirs.insert` is true (mirror `append_file_and_directory_nodes`).  
  - **Verify:** Count edges flushed on second index of same tree; fewer CONTAINS bolts.

- [x] **PERF-005** — `branch_props.clone()` per node in replay/collect  
  - **Evidence:** `indexer.rs:1497`, `1638-1641`.  
  - **Suggested fix:** `merge_branch_properties` once per file.  
  - **Verify:** `cargo test -p cortex-indexer`.

- [x] **PERF-006** — Extra clone in `call_target_pairs`  
  - **Evidence:** `edge_spill.rs:60-67`; `insert(edge.to.clone())` at `50`.  
  - **Suggested fix:** `into_call_target_pairs` with single owned pass.  
  - **Verify:** `cargo test -p cortex-indexer edge_spill`.

- [x] **PERF-007** — Per-file double spill I/O (edges + deferred nodes)  
  - **Evidence:** `indexer.rs:928-939`.  
  - **Suggested fix:** Combined NDJSON stream or inline node writes.  
  - **Verify:** Phase timings on medium repo.

- [x] **PERF-008** — Parse loop clones `ParseBatchContext` / chunk paths  
  - **Evidence:** `indexer.rs:852-886`.  
  - **Suggested fix:** `Arc<ParseBatchContext>` + slice borrows.  
  - **Verify:** Parse phase % after change.

- [x] **PERF-009** — Relationship-type splitting (`bolt_multiplier` 7.5)  
  - **Evidence:** `report_analysis` flag; `cortex-graph` per-rel UNWIND.  
  - **Suggested fix:** Multi-rel batch query per chunk (see `audit/index-perf` matrix #10 deferred).  
  - **Verify:** `bolt_multiplier` < 4; `edge_flush` wall % down.

### cortex-graph

- [x] **PERF-010** — Per-chunk per-rel-type Bolt executions  
  - **Evidence:** `writer.rs:69-76`, `client.rs:171-179`.  
  - **Suggested fix:** Batch rel types in one UNWIND where FalkorDB allows.  
  - **Verify:** `edge_flush_bolt_executions` in index JSON.

- [x] **PERF-011** — FalkorDB missing `CodeNode(id)` index warning  
  - **Evidence:** Index log WARN on connect.  
  - **Suggested fix:** Ensure `ensure_constraints` runs before bulk edge MATCH.  
  - **Verify:** No WARN; `sec_per_bolt` stable.

### cortex-parser

- [x] **PERF-012** — Signature extraction allocates many `to_string()` per symbol  
  - **Evidence:** `signature.rs` hot path (grep).  
  - **Suggested fix:** Borrow slices where summaries are ephemeral; arena or `Cow<str>`.  
  - **Verify:** Criterion parse bench (add in `cortex-benches`).

- [x] **PERF-013** — No dedicated parse-only benchmark  
  - **Evidence:** `cortex-benches/README.md` lists MCP/cache/TF-IDF only.  
  - **Suggested fix:** Add `parse_batch_benchmark` over fixture tree.  
  - **Verify:** `cargo bench --bench parse_batch_benchmark`.

### cortex-mcp

- [x] **PERF-020** — `CacheHierarchy` not used on MCP hot path  
  - **Evidence:** `capsule.rs:323` field never read in `build()`; handler duplicates capsule logic (`handler.rs:1900+`).  
  - **Suggested fix:** Shared cached capsule builder or tool-level cache with `cache_enabled` flag.  
  - **Verify:** `cache_benchmark` hit rate; telemetry `cache_hit=l1`.

- [x] **PERF-021** — L1 `invalidate_repo` wrong key prefix  
  - **Evidence:** Keys `tool:repo:hash` vs prefix `repo:` (`cache.rs:93-94`, `174-176`).  
  - **Suggested fix:** Match middle segment `repo_path`.  
  - **Verify:** `cargo test -p cortex-mcp cache`.

- [x] **PERF-022** — L1 eviction full sort O(n log n)  
  - **Evidence:** `cache.rs:208-222`.  
  - **Suggested fix:** `select_nth_unstable` for k oldest.  
  - **Verify:** `cargo test -p cortex-mcp cache`.

- [x] **PERF-023** — `VectorService::from_config` per tool call  
  - **Evidence:** `handler.rs:1942`, `vector_service.rs:91-99`.  
  - **Suggested fix:** Lazy `Arc<AsyncMutex<Option<VectorService>>>` on handler (like graph client).  
  - **Verify:** Lance open count in trace logs.

- [x] **PERF-024** — `get_context_capsule` graph + vector duplicate retrieval  
  - **Evidence:** `handler.rs:1929-2038`.  
  - **Suggested fix:** Vector-first skip `find_code` when budget filled.  
  - **Verify:** Tool latency metrics.

- [x] **PERF-025** — Memory tools JSON full-file rewrite  
  - **Evidence:** `handler.rs:6580+`, `3836-3877`.  
  - **Suggested fix:** Wire `MemoryStore` SQLite or append-only log.  
  - **Verify:** `save_observation` latency under load.

- [x] **PERF-026** — `get_impact_graph` ignores depth for traversal  
  - **Evidence:** `handler.rs:2816-2825` (always `all_callers`).  
  - **Suggested fix:** Bound Cypher/query depth from `req.depth`.  
  - **Verify:** MCP test + impact_benchmark depth cases.

### cortex-vector

- [x] **PERF-030** — Lance store open per `VectorService::from_config`  
  - **Evidence:** Same as PERF-023.  
  - **Suggested fix:** Handler-level service cache.  
  - **Verify:** Hybrid search p99.

- [x] **PERF-031** — No vector index/hybrid benchmark in `cortex-benches`  
  - **Evidence:** `cortex-benches/Cargo.toml` benches list.  
  - **Suggested fix:** Add embed + search bench with temp DB.  
  - **Verify:** `cargo bench` new target.

### cortex-watcher

- [x] **PERF-040** — Burst watch events without integration perf test  
  - **Evidence:** `perf.rs` backpressure exists; no criterion bench.  
  - **Suggested fix:** Bench `PerfManager` queue saturation.  
  - **Verify:** New bench + `cargo test -p cortex-watcher`.

### cortex-core

- [x] **PERF-050** — Document `highspeed` vs `conservative` RSS/throughput tradeoffs  
  - **Evidence:** `config.rs` indexer profiles; `audit/index-perf/README.md`.  
  - **Suggested fix:** Table in `docs/perf/` linking profiles to gates.  
  - **Verify:** Doc review only.

### cortex-analyzer

- [x] **PERF-060** — Smell scan O(files × detectors); no early exit for huge files  
  - **Evidence:** `cortex-cli` `detect_smells_in_path_with_context`.  
  - **Suggested fix:** Size cap or skip generated paths by default.  
  - **Verify:** `cortex analyze smells` on large tree timing.

### cortex-pipeline

- [x] **PERF-070** — Pipeline stages not benchmarked  
  - **Evidence:** `cortex-pipeline/src/pipeline.rs` — no benches.  
  - **Suggested fix:** Stage batching audit + micro-bench.  
  - **Verify:** TBD.

### cortex-a2a

- [x] **PERF-080** — `A2aHub` event log `Vec` under `RwLock` unbounded  
  - **Evidence:** `hub.rs:75` `events: Arc<RwLock<Vec<...>>>`.  
  - **Suggested fix:** Ring buffer or cap + spill.  
  - **Verify:** Load test `a2a_blackboard_load`.

### cortex-cli

- [x] **PERF-090** — `large_enum_variant` in CLI (clippy)  
  - **Evidence:** Clippy workspace log for `cortex-cli`.  
  - **Suggested fix:** `Box` large variants or split enums.  
  - **Verify:** `cargo clippy -p cortex-cli`.

### cortex-benches

- [x] **PERF-100** — Criterion duplicate benchmark ID in `tfidf_benchmark`  
  - **Evidence:** `cargo bench` panic: duplicated `tfidf_score/query/3_terms` (queries 2 and 3 both have len 3).  
  - **Suggested fix:** Unique `BenchmarkId` per query index.  
  - **Verify:** `cargo bench -p cortex-benches --bench tfidf_benchmark`.

- [x] **PERF-101** — Extend benches per PERF-013, PERF-031, PERF-040  
  - **Evidence:** README lists 4 benches only.  
  - **Suggested fix:** Add parse, graph-write, vector, watcher targets.  
  - **Verify:** `cargo bench -p cortex-benches`.

---

## Post-fix verification (2026-06-01)

| Metric | Pre-fix (`review-medium-highspeed-pure.json`) | Post-fix (`review-post-fix-pure.json`) |
| --- | --- | --- |
| `duration_secs` | 15.45 | 16.35 |
| `edge_flush` % | 42.3% | **25.7%** |
| `deferred_node_write` % | 23.5% | 44.6% (force+branch variance) |
| `edges_per_sec` | 11 859 | 18 483 |
| `edges_flushed` | 77 565 | 77 708 |

CONTAINS dedup (PERF-004) reduced edge-flush wall share. Backlog-fix waves add highspeed `wipe_branch_first`, binary edge spill, Falkor mixed-rel UNWIND, and MCP caches. Re-index with `cortex index . --force --format json` and analyze to refresh `review-backlog-complete-pure.json` when graph backend is available.

## Shipped in this review (code)

| ID | Change |
| --- | --- |
| PERF-001 | Binary edge spill (RMP tag + length prefix); JSON line fallback reader |
| PERF-002 | Highspeed profile sets `index_force_delete_branch_before_parse` |
| PERF-003 | Deferred spill uses `nodes_rmp` (no inline `nodes` clone in JSON) |
| PERF-004 | `spill_file_edges` CONTAINS dedup via `seen_dirs.insert` |
| PERF-005 | `merge_branch_properties` helper |
| PERF-006 | `take_call_target_pairs` (drain ids before edge flush) |
| PERF-007 | Slim deferred records + wipe_branch skips deferred path |
| PERF-008 | `Arc<ParseBatchContext>`; batch path slices without pre-cloning all chunks |
| PERF-009 | Falkor mixed-rel single UNWIND per chunk (`FOREACH` branches) |
| PERF-010 | Same mixed-rel batching in `falkordb_bulk_upsert_edges` |
| PERF-011 | Falkor schema info log after `ensure_constraints` |
| PERF-020 | L1 tool cache on `get_context_capsule` when `cache_enabled` |
| PERF-021 | L1 `invalidate_repo` key matching |
| PERF-022 | L1 `evict_oldest` partial selection |
| PERF-023 | Handler-cached `VectorService` |
| PERF-024 | Vector-first capsule; skip `find_code` when budget full |
| PERF-025 | Memory tools via SQLite `MemoryStore` + JSON migration |
| PERF-026 | `get_impact_graph` uses bounded `who_calls` depth |
| PERF-030 | Same vector service cache as PERF-023 |
| PERF-031 | `vector_smoke_benchmark` (Lance open) |
| PERF-040 | `watcher_perf_benchmark` queue saturation |
| PERF-050 | [index-profiles.md](index-profiles.md) |
| PERF-060 | Smell scan `MAX_ANALYZE_FILE_BYTES` = 512 KiB |
| PERF-070 | `pipeline_stage_benchmark` |
| PERF-080 | `A2aHub` events capped at 10_000 |
| PERF-090 | `Box<WorkspaceCommand>` (clippy `large_enum_variant`) |
| PERF-100 | `tfidf_benchmark` unique Criterion IDs |
| PERF-101 | Parse / pipeline / watcher / vector benches + README |

## Deferred (needs design)

None — backlog-fix waves landed on `perf/crates-review`.
