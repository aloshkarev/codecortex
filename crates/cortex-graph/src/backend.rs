//! FalkorDB graph backend configuration.

use cortex_core::{CortexConfig, Result};
use serde_json::Value;
use std::fmt;
use std::time::Duration;

/// Graph database backend (FalkorDB-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// FalkorDB (Redis protocol)
    FalkorDB,
}

/// Resolve backend from [`CortexConfig`]. Phase 1 always returns FalkorDB.
pub fn detect_backend_from_config(_config: &CortexConfig) -> BackendKind {
    BackendKind::FalkorDB
}

impl BackendKind {
    /// Default Redis port for FalkorDB.
    pub fn default_port(&self) -> u16 {
        match self {
            BackendKind::FalkorDB => 6379,
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendKind::FalkorDB => write!(f, "FalkorDB"),
        }
    }
}

/// Configuration for FalkorDB.
#[derive(Debug, Clone)]
pub struct BackendConfig {
    /// Backend type
    pub kind: BackendKind,
    /// Connection URI (from `CortexConfig::falkordb_uri`)
    pub uri: String,
    /// Username (unused for FalkorDB; kept for adapter compatibility)
    pub username: String,
    /// Password
    pub password: String,
    /// FalkorDB graph name
    pub database: Option<String>,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Query timeout
    pub query_timeout: Duration,
    /// Max connection pool size
    pub max_pool_size: usize,
    /// Enable SSL/TLS
    pub use_tls: bool,
    /// Custom headers (reserved for HTTP-based backends)
    pub headers: std::collections::HashMap<String, String>,
}

impl BackendConfig {
    /// Create backend configuration from cortex config.
    pub fn from_cortex_config(config: &CortexConfig) -> Self {
        Self {
            kind: BackendKind::FalkorDB,
            uri: config.falkordb_uri.clone(),
            username: String::new(),
            password: config.falkordb_password.clone(),
            database: Some(config.falkordb_graph.clone()),
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(60),
            max_pool_size: 10,
            use_tls: config.falkordb_uri.starts_with("rediss://")
                || config.falkordb_uri.starts_with("falkors://"),
            headers: std::collections::HashMap::new(),
        }
    }

    /// Set the graph name.
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Set the connection timeout.
    pub fn with_connection_timeout(mut self, timeout: Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }

    /// Set the query timeout.
    pub fn with_query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    /// Set the max pool size.
    pub fn with_max_pool_size(mut self, size: usize) -> Self {
        self.max_pool_size = size;
        self
    }

    /// Enable or disable TLS.
    pub fn with_tls(mut self, use_tls: bool) -> Self {
        self.use_tls = use_tls;
        self
    }
}

/// Query options for backend-specific tuning.
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
    /// Create default query options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the query timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Use read replica.
    pub fn use_read_replica(mut self) -> Self {
        self.use_read_replica = true;
        self
    }

    /// Set query hint.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Set batch size.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = Some(size);
        self
    }

    /// Enable profiling.
    pub fn with_profile(mut self) -> Self {
        self.profile = true;
        self
    }
}

/// Query result with metadata.
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
    /// Create a new query result.
    pub fn new(rows: Vec<Value>, backend: BackendKind) -> Self {
        Self {
            rows,
            execution_time_ms: 0,
            backend,
            from_cache: false,
            warnings: Vec::new(),
        }
    }

    /// Set the execution time.
    pub fn with_execution_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }

    /// Mark as from cache.
    pub fn from_cache(mut self) -> Self {
        self.from_cache = true;
        self
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

/// Backend statistics.
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

/// Placeholder for future health-check adapters.
pub async fn health_check_placeholder() -> Result<bool> {
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_backend_from_config_falkordb() {
        let config = CortexConfig {
            falkordb_uri: "falkor://127.0.0.1:6379".to_string(),
            ..Default::default()
        };
        assert_eq!(detect_backend_from_config(&config), BackendKind::FalkorDB);
    }

    #[test]
    fn backend_kind_default_port() {
        assert_eq!(BackendKind::FalkorDB.default_port(), 6379);
    }

    #[test]
    fn backend_kind_display() {
        assert_eq!(format!("{}", BackendKind::FalkorDB), "FalkorDB");
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
        let result = QueryResult::new(vec![], BackendKind::FalkorDB)
            .with_execution_time(50)
            .from_cache();

        assert_eq!(result.execution_time_ms, 50);
        assert!(result.from_cache);
        assert_eq!(result.backend, BackendKind::FalkorDB);
    }

    #[test]
    fn backend_config_tls_detection() {
        let config = CortexConfig {
            falkordb_uri: "rediss://localhost:6379".to_string(),
            ..Default::default()
        };

        let backend_config = BackendConfig::from_cortex_config(&config);
        assert!(backend_config.use_tls);
    }
}
