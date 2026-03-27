# cortex-graph

> `cortex-graph` is the graph database client and Cypher query engine for CodeCortex. It stores and retrieves code relationships in Memgraph, Neo4j, or AWS Neptune using type-safe, parameterized queries, and supports export/import of graph snapshots as compressed MessagePack bundles.

## What it does

- Connects to and queries Memgraph, Neo4j, or AWS Neptune via the Bolt protocol
- Provides a type-safe Cypher query builder for all relationship traversal patterns
- Manages a connection pool with health checks and configurable timeouts
- Runs versioned schema migrations (indexes and constraints) on startup
- Exports and imports graph data as `.ccx` MessagePack bundles for offline use or transfer

## Features

| Feature | Description |
|---------|-------------|
| `GraphClient` | Primary connection entry point supporting all three backends |
| `QueryEngine` | Type-safe Cypher builder for callers, callees, deps, hierarchy, and more |
| `ConnectionPool` | Managed pool via `deadpool` with configurable size and timeout |
| `MigrationManager` | Versioned schema migrations for indexes and constraints |
| `BundleStore` | Export/import graph data in MessagePack format |
| `CrossProjectQueryBuilder` | Multi-repository query composition and filtering |
| Parameterized queries | All query paths use parameters — Cypher injection is not possible |

## Supported backends

| Backend | `backend_type` value | URI format | Notes |
|---------|---------------------|-----------|-------|
| Memgraph | `memgraph` | `memgraph://host:7687` or `bolt://host:7687` | Default; recommended for local use |
| Neo4j | `neo4j` | `bolt://host:7687` or `neo4j://host:7687` | Enterprise-compatible |
| AWS Neptune | `neo4j` | `bolt://your.neptune.endpoint:8182` | Use Neo4j driver with Neptune Bolt endpoint |

## Graph schema

### Node labels

| Label | Properties |
|-------|-----------|
| `Repository` | `path`, `name`, `language` |
| `Directory` | `path`, `name` |
| `File` | `path`, `name`, `language`, `hash` |
| `Function` / `Method` | `name`, `qualified_name`, `visibility`, `file`, `line` |
| `Class` / `Struct` / `Enum` / `Trait` | `name`, `qualified_name`, `visibility`, `file`, `line` |

### Relationship types

| Type | Source → Target | Description |
|------|----------------|-------------|
| `CONTAINS` | Directory/File → child | Hierarchical containment |
| `CALLS` | Function → Function | Function or method call |
| `IMPORTS` | File → Symbol | Import or `use` statement |
| `INHERITS` | Class → Class | Inheritance |
| `IMPLEMENTS` | Class/Struct → Trait | Trait or interface implementation |
| `MEMBER_OF` | Method/Field → Type | Member-to-parent type relationship |
| `TYPE_REFERENCE` | Symbol → Type | Type used in a type-position |
| `FIELD_ACCESS` | Expression → Field | Field access expression |

Schema indexes are maintained on `qualified_name` for fast symbol resolution during navigation queries.

## Usage

### Connect to database

```rust
use cortex_graph::GraphClient;
use cortex_core::CortexConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = CortexConfig::load()?;
    let client = GraphClient::connect(&config).await?;
    Ok(())
}
```

### Connection pooling

```rust
use cortex_graph::pool::{ConnectionPool, PoolConfig};
use cortex_core::CortexConfig;
use std::time::Duration;

let db_config = CortexConfig::load()?;
let pool_config = PoolConfig {
    max_connections: 10,
    min_idle: 2,
    connection_timeout: Duration::from_secs(30),
    ..Default::default()
};
let pool = ConnectionPool::new(db_config, pool_config);
let conn = pool.get().await?;
```

### Export and import bundles

```rust
use cortex_graph::{GraphClient, BundleStore};
use std::path::Path;

// Export
let bundle = BundleStore::export_from_graph(&client, "/path/to/repo").await?;
BundleStore::export(Path::new("export.ccx"), &bundle)?;

// Import
let bundle = BundleStore::import(Path::new("export.ccx"))?;
BundleStore::import_to_graph(&client, &bundle).await?;
```

### Schema migration

```rust
use cortex_graph::migration::{MigrationManager, CURRENT_VERSION};

let manager = MigrationManager::new();
let result = manager.migrate(&client).await?;
println!("Migrated to version {}", result.version);
```

### Security: parameterized queries

All query methods use parameterized queries to prevent Cypher injection:

```rust
// Safe: parameterized query
let results = engine.callers("user_input").await?;

// Also safe: explicit parameter binding
let results = client
    .query_with_param("MATCH (n) WHERE n.name = $name RETURN n", "name", user_input)
    .await?;
```

## Dependencies

- `neo4rs` — Neo4j/Memgraph Bolt driver
- `rsmgclient` — Memgraph native client
- `deadpool` — Async connection pooling
- `rmp-serde` — MessagePack serialization (bundle format)
- `serde` / `serde_json` — Serialization

## Tests

```bash
cargo test -p cortex-graph -- --test-threads=1
```

Current test count: **84 tests**
