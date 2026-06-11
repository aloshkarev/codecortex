# cortex-graph

Graph database client and query engine for code intelligence.

## Overview

This crate provides FalkorDB integration for storing and querying code relationships.

## Features

- **Graph Client**: Connection to FalkorDB via Redis RESP (`GRAPH.QUERY`)
- **Query Engine**: Type-safe Cypher query building
- **Bundle Store**: Export/import graph data in MessagePack format
- **Schema Management**: Versioned migrations for indexes and constraints
- **Connection Pool**: Managed database connections with health checks
- **Bulk indexing**: `UNWIND` batch upserts with configurable write pool
- **Query Timeouts**: Configurable timeout handling with retry logic

## Usage

### Connecting to Database

```rust
use cortex_graph::GraphClient;
use cortex_core::CortexConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CortexConfig::default();
    let client = GraphClient::connect(&config).await?;
    // Execute queries...
    Ok(())
}
```

### Connection Pooling

```rust
use cortex_graph::pool::{ConnectionPool, PoolConfig};
use cortex_core::CortexConfig;
use std::time::Duration;

let db_config = CortexConfig::default();
let pool_config = PoolConfig {
    max_connections: 10,
    min_idle: 2,
    connection_timeout: Duration::from_secs(30),
    ..Default::default()
};
let pool = ConnectionPool::new(db_config, pool_config);
let conn = pool.get().await?;
```

### Exporting Data

```rust
use cortex_graph::{GraphClient, BundleStore};
use std::path::Path;

async fn export_example(client: &GraphClient) -> Result<(), Box<dyn std::error::Error>> {
    let bundle = BundleStore::export_from_graph(client, "/path/to/repo").await?;
    BundleStore::export(Path::new("export.ccx"), &bundle)?;
    Ok(())
}
```

### Schema Migrations

```rust
use cortex_graph::migration::{MigrationManager, CURRENT_VERSION};

let manager = MigrationManager::new();
let result = manager.migrate(&client).await?;
println!("Migrated to version {}", result.version);
```

## Schema

The graph uses the following node labels:

- `Repository` - Root repository nodes
- `Directory` - Directory structure
- `File` - Source files
- `Function`, `Class`, `Struct`, `Enum`, `Trait` - Code entities

Relationship types:

- `CONTAINS` - Hierarchical containment
- `CALLS` - Function calls
- `IMPORTS` - Import statements
- `INHERITS` - Class inheritance
- `IMPLEMENTS` - Trait implementations
- `MEMBER_OF` - Member-to-parent type relationship
- `TYPE_REFERENCE` - Type-position references
- `FIELD_ACCESS` - Field access expressions

## Recent updates

- Added `CrossProjectQueryBuilder` for multi-repository query composition and filtering.
- Added navigation-oriented schema indexes on `qualified_name` for fast symbol resolution.
- `GraphClient` now includes type-reference resolution and promotes key node properties (`qualified_name`, `visibility`) for query efficiency.

## Configuration

Set FalkorDB connection details in `~/.cortex/config.toml`:

```toml
backend_type = "falkordb"
falkordb_uri = "falkor://127.0.0.1:6379"
falkordb_graph = "codecortex"
falkordb_write_pool_size = 4
```

## Tests

Run tests with:

```bash
cargo test -p cortex-graph -- --test-threads=1
```

Live FalkorDB tests (requires Docker):

```bash
RUN_DOCKER_INTEGRATION=1 CORTEX_TEST_GRAPH_URI=falkor://127.0.0.1:6379 \
  cargo test -p cortex-graph --test falkordb_bulk_upsert_smoke -- --ignored
```

## Security

All query methods use parameterized queries to prevent Cypher injection:

```rust
// Safe: uses parameterized query
let results = engine.callers("user_input").await?;

// Also safe: GraphClient methods
let results = client
    .query_with_param("MATCH (n) WHERE n.name = $name RETURN n", "name", user_input)
    .await?;
```
