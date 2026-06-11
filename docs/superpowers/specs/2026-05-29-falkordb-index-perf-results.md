# FalkorDB index performance results

Generated: 2026-05-28T22:42:20Z
Repo: `/run/media/alex/artefacts/projects/self/projects/64-codecortex/crates/cortex-parser/tests/fixtures/sample_project_rust`

| case | backend | batch | source_cap | duration_secs | phase_edge_flush | edges | edges_per_sec | bolt_exec | falkor_max_query_bytes | falkor_lock_wait_frac |
|------|---------|-------|------------|---------------|------------------|-------|---------------|-----------|------------------------|----------------------|
| falkordb_baseline | falkordb | 2048 | 262144 | 0.139541497 | 0.004020903 | 9 | 2238.3 | 3 | 2803 | 0.0030388978930307943 |
| falkordb_b1024 | falkordb | 1024 | 262144 | 0.138578131 | 0.003830888 | 9 | 2349.3 | 3 | 2803 | 0.003488372093023256 |
| falkordb_b4096 | falkordb | 4096 | 262144 | 0.139139811 | 0.003911001 | 9 | 2301.2 | 3 | 2803 | 0.0031515483694162785 |
| falkordb_src0 | falkordb | 2048 | 0 | 0.142963036 | 0.003653141 | 9 | 2463.6 | 3 | 2803 | 0.004550171629280754 |
| falkordb_src32768 | falkordb | 2048 | 32768 | 0.137526115 | 0.003674155 | 9 | 2449.5 | 3 | 2803 | 0.0035753147831493856 |
| memgraph_baseline | memgraph | 2048 | 262144 |  |  |  | n/a |  |  |  |
| memgraph_pool4 | memgraph | 2048 | 262144 |  |  |  | n/a |  |  |  |

Analyze: `cortex index-report analyze --file report.json`
See [hypothesis report](2026-05-29-falkordb-index-perf-analysis.md).
