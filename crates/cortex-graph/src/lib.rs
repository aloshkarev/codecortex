//! # CodeCortex Graph Library
//!
//! Graph database client and query engine for code intelligence.
//!
//! ## Overview
//!
//! This crate provides graph database integration:
//!
//! - **Graph Client**: [`GraphClient`] for Neo4j/Memgraph connections
//! - **Query Engine**: [`QueryEngine`] for building and executing Cypher queries
//! - **Bundle Store**: [`BundleStore`] for exporting/importing graph data
//! - **Node Writer**: [`NodeWriter`] for batch node insertion
//! - **Connection Pool**: [`ConnectionPool`] for managing database connections
//! - **Schema Migrations**: [`MigrationManager`] for versioned schema changes
//! - **Multiple Backends**: [`BackendKind`] for Neo4j, Memgraph, Neptune support
//! - **Query Timeouts**: [`TimeoutExecutor`] for query timeout handling
//!
//! ## Connection
//!
//! The client connects to Neo4j or Memgraph databases using bolt protocol:
//!
//! ```rust,no_run
//! use cortex_graph::GraphClient;
//! use cortex_core::CortexConfig;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = CortexConfig::default();
//!     let client = GraphClient::connect(&config).await?;
//!     // Execute queries...
//!     Ok(())
//! }
//! ```
//!
//! ## Bundle Format
//!
//! Graph data can be exported to `.ccx` files (MessagePack format):
//!
//! ```rust,no_run
//! use cortex_graph::{GraphClient, BundleStore};
//! use std::path::Path;
//!
//! # async fn example(client: &GraphClient) -> Result<(), Box<dyn std::error::Error>> {
//! let bundle = BundleStore::export_from_graph(client, "/path/to/repo").await?;
//! BundleStore::export(Path::new("export.ccx"), &bundle)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Connection Pooling
//!
//! ```rust,no_run
//! use cortex_graph::pool::{ConnectionPool, PoolConfig};
//! use cortex_core::CortexConfig;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let db_config = CortexConfig::default();
//!     let pool_config = PoolConfig {
//!         max_connections: 10,
//!         min_idle: 2,
//!         connection_timeout: Duration::from_secs(30),
//!         ..Default::default()
//!     };
//!     let pool = ConnectionPool::new(db_config, pool_config);
//!
//!     let conn = pool.get().await?;
//!     // Use conn.client()...
//!     Ok(())
//! }
//! ```

pub mod backend;
pub mod bundle;
pub mod client;
pub mod cross_project;
pub mod memgraph;
pub mod migration;
pub mod pool;
pub mod query_engine;
pub mod schema;
pub mod scoped_query;
pub mod timeout;
pub mod writer;

pub use backend::{BackendConfig, BackendKind, BackendStats, QueryOptions, QueryResult};
pub use bundle::BundleStore;
pub use client::GraphClient;
pub use cross_project::CrossProjectQueryBuilder;
pub use memgraph::MemgraphClient;
pub use migration::{CURRENT_VERSION, MIGRATIONS, Migration, MigrationManager, MigrationResult};
pub use pool::{ConnectionPool, PoolConfig, PoolStats, PooledConnection};
pub use query_engine::{AnalysisQuery, QueryEngine};
pub use schema::{
    BranchIndexRecord, create_branch_index, delete_branch_index, ensure_constraints,
    ensure_navigation_schema, get_branch_indexes, is_branch_index_current, mark_branch_index_stale,
};
pub use scoped_query::{QueryScope, ScopedQueryBuilder, ScopedResult};
pub use timeout::{QueryTiming, QueryType, TimeoutConfig, TimeoutExecutor};
pub use writer::NodeWriter;
