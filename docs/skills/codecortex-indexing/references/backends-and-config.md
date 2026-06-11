# Graph backend and config

CodeCortex uses **FalkorDB** as the graph backend. The driver speaks Redis RESP (`GRAPH.QUERY`).

## Quick start (Docker)

```bash
docker run -d --name codecortex-falkordb -p 6379:6379 falkordb/falkordb:latest
```

`~/.cortex/config.toml`:

```toml
backend_type = "falkordb"
falkordb_uri = "falkor://127.0.0.1:6379"
falkordb_graph = "codecortex"
falkordb_password = ""
max_batch_size = 4096
```

`max_batch_size` and `falkordb_unwind_batch_max` control bulk `UNWIND` batching (fewer round trips per index run). `falkordb_write_pool_size` controls parallel `GRAPH.QUERY` connections for bulk writes.

## Verify

```bash
cortex doctor
```

MCP: `check_health`, then `diagnose` if doctor-equivalent checks fail.

## Index performance notes

- Graph writes batch via configured `max_batch_size` / `falkordb_unwind_batch_max`.
- Incremental indexing uses Git changed files; deletions trigger full rebuild.
- See [FALKORDB.md](../../../FALKORDB.md) and root `README.md` for tuning and profiling.
