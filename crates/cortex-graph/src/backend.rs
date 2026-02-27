//! Multiple Graph Backend Support
//!
//! This module provides an abstraction layer for different graph database backends.
//! Currently supports Neo4j and Memgraph with the same interface.

use cortex_core::{CortexConfig, Result};
use serde_json::Value;
use std::fmt;
use std::time::Duration;

/// Supported graph database backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// Neo4j database
    Neo4j,
    /// Memgraph database
    Memgraph,
    /// Amazon Neptune
    Neptune,
    /// Custom/other backend
    Other,
}

impl BackendKind {
    /// Detect backend type from URI
    pub fn from_uri(uri: &str) -> Self {
        let uri_lower = uri.to_lowercase();

        // Check for specific backends by name first (higher priority)
        if uri_lower.contains("memgraph") {
            return BackendKind::Memgraph;
        }

        if uri_lower.contains("neo4j") {
            return BackendKind::Neo4j;
        }

        if uri_lower.contains("neptune") {
            return BackendKind::Neptune;
        }

        // Fall back to port-based detection
        if uri.contains(":7687") {
            BackendKind::Neo4j // Default to Neo4j for bolt protocol
        } else if uri.contains(":8182") {
            BackendKind::Neptune
        } else {
            BackendKind::Other
        }
    }

    /// Get the default port for this backend
    pub fn default_port(&self) -> u16 {
        match self {
            BackendKind::Neo4j => 7687,
            BackendKind::Memgraph => 7687,
            BackendKind::Neptune => 8182,
            BackendKind::Other => 7687,
        }
    }

    /// Check if this backend supports a specific feature
    pub fn supports_feature(&self, feature: BackendFeature) -> bool {
        match self {
            BackendKind::Neo4j => matches!(
                feature,
                BackendFeature::Transactions
                    | BackendFeature::Constraints
                    | BackendFeature::Indexes
                    | BackendFeature::StoredProcedures
                    | BackendFeature::APOC
            ),
            BackendKind::Memgraph => matches!(
                feature,
                BackendFeature::Transactions
                    | BackendFeature::Constraints
                    | BackendFeature::Indexes
                    | BackendFeature::QueryModules
            ),
            BackendKind::Neptune => matches!(
                feature,
                BackendFeature::Transactions | BackendFeature::Indexes | BackendFeature::Gremlin
            ),
            BackendKind::Other => true,
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendKind::Neo4j => write!(f, "Neo4j"),
            BackendKind::Memgraph => write!(f, "Memgraph"),
            BackendKind::Neptune => write!(f, "Neptune"),
            BackendKind::Other => write!(f, "Other"),
        }
    }
}

/// Features that may be supported by different backends
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendFeature {
    /// ACID transactions
    Transactions,
    /// Unique constraints
    Constraints,
    /// Indexes
    Indexes,
    /// Stored procedures (Neo4j)
    StoredProcedures,
    /// APOC library (Neo4j)
    APOC,
    /// Query modules (Memgraph)
    QueryModules,
    /// Gremlin queries (Neptune)
    Gremlin,
}

/// Configuration for a specific backend
#[derive(Debug, Clone)]
pub struct BackendConfig {
    /// Backend type
    pub kind: BackendKind,
    /// Connection URI
    pub uri: String,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Database name (for multi-database support)
    pub database: Option<String>,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Query timeout
    pub query_timeout: Duration,
    /// Max connection pool size
    pub max_pool_size: usize,
    /// Enable SSL/TLS
    pub use_tls: bool,
    /// Custom headers (for HTTP-based backends)
    pub headers: std::collections::HashMap<String, String>,
}

impl BackendConfig {
    /// Create a new backend configuration from cortex config
    pub fn from_cortex_config(config: &CortexConfig) -> Self {
        let kind = BackendKind::from_uri(&config.memgraph_uri);
        Self {
            kind,
            uri: config.memgraph_uri.clone(),
            username: config.memgraph_user.clone(),
            password: config.memgraph_password.clone(),
            database: None,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
            max_pool_size: 10,
            use_tls: config.memgraph_uri.starts_with("bolt+s")
                || config.memgraph_uri.starts_with("neo4j+s"),
            headers: std::collections::HashMap::new(),
        }
    }

    /// Set the database name
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set the connection timeout
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set the query timeout
    pub fn with_query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    /// Set the max pool size
    pub fn with_max_pool_size(mut self, size: usize) -> Self {
        self.max_pool_size = size;
        self
    }

    /// Enable or disable TLS
    pub fn with_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }
}

/// Query options for backend-specific tuning
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    /// Query timeout override
    pub timeout: Option<Duration>,
    /// Whether to use read replica if available
    pub use_read_replica: bool,
    /// Query hint (backend-specific)
    pub hint: Option<String>,
    /// Batch size for streaming results
    pub batch_size: Option<usize>,
    /// Whether to include query profile
    pub profile: bool,
}

impl QueryOptions {
    /// Create default query options
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the query timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Use read replica
    pub fn use_read_replica(mut self) -> Self {
        self.use_read_replica = true;
        self
    }

    /// Set query hint
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = Some(size);
        self
    }

    /// Enable profiling
    pub fn with_profile(mut self) -> Self {
        self.profile = true;
        self
    }
}

/// Query result with metadata
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Result rows
    pub rows: Vec<Value>,
    /// Query execution time
    pub execution_time_ms: u64,
    /// Backend that executed the query
    pub backend: BackendKind,
    /// Whether the query was served from cache
    pub from_cache: bool,
    /// Any warnings generated
    pub warnings: Vec<String>,
}

impl QueryResult {
    /// Create a new query result
    pub fn new(rows: Vec<Value>, backend: BackendKind) -> Self {
        Self {
            rows,
            execution_time_ms: 0,
            backend,
            from_cache: false,
            warnings: Vec::new(),
        }
    }

    /// Set the execution time
    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }

    /// Mark as from cache
    pub fn from_cache(mut self) -> Self {
        self.from_cache = true;
        self
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Backend adapter trait for different graph databases
#[async_trait::async_trait]
pub trait BackendAdapter: Send + Sync {
    /// Get the backend type
    fn backend_kind(&self) -> BackendKind;

    /// Execute a query
    async fn execute(&self, query: &str, options: QueryOptions) -> Result<QueryResult>;

    /// Execute a query with parameters
    async fn execute_with_params(
        &self,
        query: &str,
        params: Value,
        options: QueryOptions,
    ) -> Result<QueryResult>;

    /// Begin a transaction
    async fn begin_transaction(&self) -> Result<Box<dyn Transaction>>;

    /// Health check
    async fn health_check(&self) -> Result<bool>;

    /// Get backend statistics
    async fn stats(&self) -> Result<BackendStats>;
}

/// Transaction trait for backend transactions
#[async_trait::async_trait]
pub trait Transaction: Send {
    /// Execute a query in the transaction
    async fn execute(&mut self, query: &str) -> Result<QueryResult>;

    /// Commit the transaction
    async fn commit(self: Box<Self>) -> Result<()>;

    /// Rollback the transaction
    async fn rollback(self: Box<Self>) -> Result<()>;
}

/// Backend statistics
#[derive(Debug, Clone, Default)]
pub struct BackendStats {
    /// Number of nodes
    pub node_count: u64,
    /// Number of relationships
    pub relationship_count: u64,
    /// Number of labels
    pub label_count: u64,
    /// Number of relationship types
    pub relationship_type_count: u64,
    /// Database size in bytes
    pub database_size_bytes: u64,
    /// Number of indexes
    pub index_count: u64,
    /// Number of constraints
    pub constraint_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_kind_from_uri_neo4j() {
        assert_eq!(
            BackendKind::from_uri("bolt://neo4j.example.com:7687"),
            BackendKind::Neo4j
        );
        assert_eq!(
            BackendKind::from_uri("neo4j://localhost:7687"),
            BackendKind::Neo4j
        );
    }

    #[test]
    fn backend_kind_from_uri_memgraph() {
        assert_eq!(
            BackendKind::from_uri("bolt://memgraph.example.com:7687"),
            BackendKind::Memgraph
        );
    }

    #[test]
    fn backend_kind_from_uri_neptune() {
        assert_eq!(
            BackendKind::from_uri("https://neptune.cluster.amazonaws.com:8182"),
            BackendKind::Neptune
        );
    }

    #[test]
    fn backend_kind_default_ports() {
        assert_eq!(BackendKind::Neo4j.default_port(), 7687);
        assert_eq!(BackendKind::Memgraph.default_port(), 7687);
        assert_eq!(BackendKind::Neptune.default_port(), 8182);
    }

    #[test]
    fn backend_kind_display() {
        assert_eq!(format!("{}", BackendKind::Neo4j), "Neo4j");
        assert_eq!(format!("{}", BackendKind::Memgraph), "Memgraph");
        assert_eq!(format!("{}", BackendKind::Neptune), "Neptune");
    }

    #[test]
    fn backend_features_neo4j() {
        assert!(BackendKind::Neo4j.supports_feature(BackendFeature::Transactions));
        assert!(BackendKind::Neo4j.supports_feature(BackendFeature::Constraints));
        assert!(BackendKind::Neo4j.supports_feature(BackendFeature::APOC));
        assert!(!BackendKind::Neo4j.supports_feature(BackendFeature::QueryModules));
    }

    #[test]
    fn backend_features_memgraph() {
        assert!(BackendKind::Memgraph.supports_feature(BackendFeature::Transactions));
        assert!(BackendKind::Memgraph.supports_feature(BackendFeature::QueryModules));
        assert!(!BackendKind::Memgraph.supports_feature(BackendFeature::APOC));
    }

    #[test]
    fn query_options_builder() {
        let opts = QueryOptions::new()
            .with_timeout(Duration::from_secs(30))
            .use_read_replica()
            .with_batch_size(100);

        assert_eq!(opts.timeout, Some(Duration::from_secs(30)));
        assert!(opts.use_read_replica);
        assert_eq!(opts.batch_size, Some(100));
    }

    #[test]
    fn query_result_builder() {
        let result = QueryResult::new(vec![], BackendKind::Neo4j)
            .with_execution_time(50)
            .from_cache();

        assert_eq!(result.execution_time_ms, 50);
        assert!(result.from_cache);
        assert_eq!(result.backend, BackendKind::Neo4j);
    }

    #[test]
    fn backend_config_tls_detection() {
        let config = CortexConfig {
            memgraph_uri: "bolt+s://localhost:7687".to_string(),
            ..Default::default()
        };

        let backend_config = BackendConfig::from_cortex_config(&config);
        assert!(backend_config.use_tls);
    }
}
