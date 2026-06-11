# FalkorDB default backend — implementation notes

## Parameter strategy (Phase 0)

The `falkordb` Rust crate 0.2.1 fails to compile (`queries overflow the depth limit`). CodeCortex uses the `redis` crate with `GRAPH.QUERY` instead.

FalkorDB `CYPHER key=value` string parameters do not carry Bolt-style typed map batches. Bulk indexer paths use:

1. **Inline literals** for `QueryParam::List` parameters referenced as `$batch`, `$paths`, `$ids` in Cypher (`falkordb_params::prepare_cypher_query`).
2. **CYPHER prefixes** for scalar parameters (`repository_path`, `branch`, etc.).

Validated by unit tests and `falkordb_bulk_upsert_smoke` (testcontainers + `falkordb/falkordb:latest`).

## Defaults

- `CortexConfig::default().backend_type = "falkordb"`
- `memgraph_uri = "falkor://127.0.0.1:6379"` (field name kept for compatibility)
- `falkordb_graph = "codecortex"`
