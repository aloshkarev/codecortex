//! # CodeCortex Graph Library
//!
//! Graph database client and query engine for code intelligence (FalkorDB-only).
//!
//! ## Overview
//!
//! This crate provides graph database integration:
//!
//! - **Graph Client**: [`GraphClient`] for FalkorDB connections
//! - **Query Engine**: [`QueryEngine`] for building and executing Cypher queries
//! - **Bundle Store**: [`BundleStore`] for exporting/importing graph data
//! - **Node Writer**: [`NodeWriter`] for batch node insertion
//! - **Connection Pool**: [`ConnectionPool`] for managing database connections
//! - **Schema Migrations**: [`MigrationManager`] for versioned schema changes
//! - **Query Timeouts**: [`TimeoutExecutor`] for query timeout handling
//!
//! ## Connection
//!
//! The client connects to FalkorDB using the Redis protocol:
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
pub mod blackboard;
pub mod bundle;
pub mod client;
pub mod cross_project;
mod edge_profile;
pub mod falkordb;
pub mod falkordb_params;
pub mod falkordb_profile;
pub mod migration;
pub mod pool;
pub mod query_engine;
pub mod schema;
pub mod scoped_query;
pub mod timeout;
pub mod writer;

pub use backend::{
    BackendConfig, BackendKind, BackendStats, QueryOptions, QueryResult, detect_backend_from_config,
};
pub use blackboard::{AgentInsightRecord, BlackboardWriter, insight_id};
pub use bundle::BundleStore;
pub use client::GraphClient;
pub use cross_project::CrossProjectQueryBuilder;
pub use edge_profile::{EdgeWriteProfile, RelTypeBoltStats};
pub use falkordb::FalkorDbClient;
pub use falkordb_params::GraphParam;
pub use falkordb_profile::{FalkorDbProfileSnapshot, falkordb_profile_enabled};
pub use migration::{CURRENT_VERSION, MIGRATIONS, Migration, MigrationManager, MigrationResult};
pub use pool::{ConnectionPool, PoolConfig, PoolStats, PooledConnection};
pub use query_engine::{AnalysisQuery, QueryEngine};
pub use schema::{
    BranchIndexRecord, clear_file_tombstone, create_branch_index, delete_branch_index,
    delete_file_index, ensure_constraints, ensure_navigation_schema, get_branch_indexes,
    is_branch_index_current, mark_branch_index_stale, mark_branch_vector_fresh,
    upsert_file_tombstone, warn_if_falkordb_codenode_id_index_missing,
};
pub use scoped_query::{QueryScope, ScopedQueryBuilder, ScopedResult};
pub use timeout::{QueryTiming, QueryType, TimeoutConfig, TimeoutExecutor};
pub use writer::NodeWriter;
