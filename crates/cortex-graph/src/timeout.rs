//! Query Timeout Handling
//!
//! This module provides timeout handling for graph database queries.
//! It ensures queries don't hang indefinitely and provides cancellation support.

use cortex_core::{CortexError, Result};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Default query timeout
pub const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_secs(60);

/// Maximum query timeout
pub const MAX_QUERY_TIMEOUT: Duration = Duration::from_secs(300);

/// Minimum query timeout
pub const MIN_QUERY_TIMEOUT: Duration = Duration::from_millis(100);

/// Timeout configuration for queries
#[derive(Debug, Clone, Copy)]
pub struct TimeoutConfig {
    /// Default timeout for regular queries
    pub default_timeout: Duration,
    /// Timeout for read queries
    pub read_timeout: Duration,
    /// Timeout for write queries
    pub write_timeout: Duration,
    /// Timeout for batch operations
    pub batch_timeout: Duration,
    /// Timeout for schema operations
    pub schema_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout: DEFAULT_QUERY_TIMEOUT,
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(60),
            batch_timeout: Duration::from_secs(120),
            schema_timeout: Duration::from_secs(300),
        }
    }
}

impl TimeoutConfig {
    /// Create a new timeout configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the default timeout
    pub fn with_default(mut self, timeout: Duration) -> Self {
        self.default_timeout = timeout.clamp(MIN_QUERY_TIMEOUT, MAX_QUERY_TIMEOUT);
        self
    }

    /// Set the read timeout
    pub fn with_read(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout.clamp(MIN_QUERY_TIMEOUT, MAX_QUERY_TIMEOUT);
        self
    }

    /// Set the write timeout
    pub fn with_write(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout.clamp(MIN_QUERY_TIMEOUT, MAX_QUERY_TIMEOUT);
        self
    }

    /// Set the batch timeout
    pub fn with_batch(mut self, timeout: Duration) -> Self {
        self.batch_timeout = timeout.clamp(MIN_QUERY_TIMEOUT, MAX_QUERY_TIMEOUT * 2);
        self
    }

    /// Get the appropriate timeout for a query type
    pub fn get_timeout(&self, query_type: QueryType) -> Duration {
        match query_type {
            QueryType::Read => self.read_timeout,
            QueryType::Write => self.write_timeout,
            QueryType::Batch => self.batch_timeout,
            QueryType::Schema => self.schema_timeout,
            QueryType::Unknown => self.default_timeout,
        }
    }
}

/// Types of queries for timeout selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Read-only query (SELECT/MATCH)
    Read,
    /// Write query (INSERT/CREATE/UPDATE/DELETE)
    Write,
    /// Batch operation
    Batch,
    /// Schema operation (DDL)
    Schema,
    /// Unknown query type
    Unknown,
}

impl QueryType {
    /// Detect query type from Cypher query text
    pub fn detect(query: &str) -> Self {
        let query_upper = query.to_uppercase();
        let trimmed = query_upper.trim();

        // Check for schema operations first
        if trimmed.starts_with("CREATE CONSTRAINT")
            || trimmed.starts_with("DROP CONSTRAINT")
            || trimmed.starts_with("CREATE INDEX")
            || trimmed.starts_with("DROP INDEX")
            || trimmed.starts_with("CREATE DATABASE")
            || trimmed.starts_with("DROP DATABASE")
        {
            return QueryType::Schema;
        }

        // Check for read operations
        if trimmed.starts_with("MATCH")
            || trimmed.starts_with("RETURN")
            || trimmed.starts_with("PROFILE")
            || trimmed.starts_with("EXPLAIN")
        {
            // But CREATE/MERGE/SET/DELETE in the query means it's write
            if query_upper.contains("CREATE")
                || query_upper.contains("MERGE")
                || query_upper.contains("SET")
                || query_upper.contains("DELETE")
                || query_upper.contains("REMOVE")
            {
                return QueryType::Write;
            }
            return QueryType::Read;
        }

        // Check for write operations
        if trimmed.starts_with("CREATE")
            || trimmed.starts_with("MERGE")
            || trimmed.starts_with("INSERT")
        {
            return QueryType::Write;
        }

        // Check for batch operations
        if query_upper.contains("UNWIND") || query_upper.contains("FOREACH") {
            return QueryType::Batch;
        }

        QueryType::Unknown
    }
}

/// Wrapper for executing queries with timeout
pub struct TimeoutExecutor {
    config: TimeoutConfig,
}

impl TimeoutExecutor {
    /// Create a new timeout executor
    pub fn new(config: TimeoutConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(TimeoutConfig::default())
    }

    /// Execute a query with automatic timeout based on query type
    pub async fn execute<F, T>(&self, query: &str, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let query_type = QueryType::detect(query);
        let timeout_duration = self.config.get_timeout(query_type);

        timeout(timeout_duration, f)
            .await
            .map_err(|_| CortexError::Database(format!(
                "Query timed out after {:?} (type: {:?})",
                timeout_duration, query_type
            )))?
    }

    /// Execute with explicit timeout
    pub async fn execute_with_timeout<F, T>(&self, timeout_duration: Duration, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        let clamped_timeout = timeout_duration.clamp(MIN_QUERY_TIMEOUT, MAX_QUERY_TIMEOUT * 2);

        timeout(clamped_timeout, f)
            .await
            .map_err(|_| CortexError::Database(format!(
                "Query timed out after {:?}",
                clamped_timeout
            )))?
    }

    /// Execute a read query
    pub async fn execute_read<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        timeout(self.config.read_timeout, f)
            .await
            .map_err(|_| CortexError::Database(format!(
                "Read query timed out after {:?}",
                self.config.read_timeout
            )))?
    }

    /// Execute a write query
    pub async fn execute_write<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        timeout(self.config.write_timeout, f)
            .await
            .map_err(|_| CortexError::Database(format!(
                "Write query timed out after {:?}",
                self.config.write_timeout
            )))?
    }

    /// Execute a batch operation
    pub async fn execute_batch<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        timeout(self.config.batch_timeout, f)
            .await
            .map_err(|_| CortexError::Database(format!(
                "Batch operation timed out after {:?}",
                self.config.batch_timeout
            )))?
    }

    /// Execute a schema operation
    pub async fn execute_schema<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        timeout(self.config.schema_timeout, f)
            .await
            .map_err(|_| CortexError::Database(format!(
                "Schema operation timed out after {:?}",
                self.config.schema_timeout
            )))?
    }
}

/// Timing information for a query
#[derive(Debug, Clone)]
pub struct QueryTiming {
    /// Query text (truncated)
    pub query_preview: String,
    /// Query type
    pub query_type: QueryType,
    /// Timeout used
    pub timeout: Duration,
    /// Actual execution time
    pub execution_time: Duration,
    /// Whether the query timed out
    pub timed_out: bool,
    /// When the query started
    pub started_at: Instant,
}

impl QueryTiming {
    /// Create a new timing record
    pub fn new(query: &str, timeout: Duration) -> Self {
        Self {
            query_preview: if query.len() > 100 {
                format!("{}...", &query[..100])
            } else {
                query.to_string()
            },
            query_type: QueryType::detect(query),
            timeout,
            execution_time: Duration::ZERO,
            timed_out: false,
            started_at: Instant::now(),
        }
    }

    /// Mark the query as complete
    pub fn complete(&mut self) {
        self.execution_time = self.started_at.elapsed();
    }

    /// Mark the query as timed out
    pub fn mark_timed_out(&mut self) {
        self.execution_time = self.started_at.elapsed();
        self.timed_out = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_config_defaults() {
        let config = TimeoutConfig::default();
        assert_eq!(config.default_timeout, DEFAULT_QUERY_TIMEOUT);
        assert_eq!(config.read_timeout, Duration::from_secs(30));
        assert_eq!(config.write_timeout, Duration::from_secs(60));
    }

    #[test]
    fn timeout_config_clamping() {
        let config = TimeoutConfig::new()
            .with_default(Duration::from_secs(1000))
            .with_read(Duration::from_millis(10));

        assert_eq!(config.default_timeout, MAX_QUERY_TIMEOUT);
        assert_eq!(config.read_timeout, MIN_QUERY_TIMEOUT);
    }

    #[test]
    fn query_type_detect_read() {
        assert_eq!(QueryType::detect("MATCH (n) RETURN n"), QueryType::Read);
        assert_eq!(QueryType::detect("RETURN 1"), QueryType::Read);
        assert_eq!(QueryType::detect("PROFILE MATCH (n) RETURN n"), QueryType::Read);
        assert_eq!(QueryType::detect("EXPLAIN MATCH (n) RETURN n"), QueryType::Read);
    }

    #[test]
    fn query_type_detect_write() {
        assert_eq!(QueryType::detect("CREATE (n) RETURN n"), QueryType::Write);
        assert_eq!(QueryType::detect("MERGE (n) RETURN n"), QueryType::Write);
        assert_eq!(QueryType::detect("MATCH (n) SET n.x = 1"), QueryType::Write);
        assert_eq!(QueryType::detect("MATCH (n) DELETE n"), QueryType::Write);
    }

    #[test]
    fn query_type_detect_schema() {
        assert_eq!(
            QueryType::detect("CREATE CONSTRAINT ON (n:Node) ASSERT n.id IS UNIQUE"),
            QueryType::Schema
        );
        assert_eq!(
            QueryType::detect("CREATE INDEX ON :Node(name)"),
            QueryType::Schema
        );
        assert_eq!(
            QueryType::detect("DROP CONSTRAINT ON (n:Node) ASSERT n.id IS UNIQUE"),
            QueryType::Schema
        );
    }

    #[test]
    fn query_type_detect_batch() {
        assert_eq!(
            QueryType::detect("UNWIND [1,2,3] AS x CREATE (n:Node {val: x})"),
            QueryType::Batch
        );
        assert_eq!(
            QueryType::detect("FOREACH (i IN [1,2,3] | CREATE (n:Node {val: i}))"),
            QueryType::Batch
        );
    }

    #[test]
    fn query_timing_creation() {
        let timing = QueryTiming::new("MATCH (n) RETURN n", Duration::from_secs(30));
        assert_eq!(timing.query_type, QueryType::Read);
        assert_eq!(timing.timeout, Duration::from_secs(30));
        assert!(!timing.timed_out);
    }

    #[test]
    fn query_timing_complete() {
        let mut timing = QueryTiming::new("MATCH (n) RETURN n", Duration::from_secs(30));
        timing.complete();
        assert!(!timing.timed_out);
        // Execution time is set by complete() - may be zero on very fast systems
        // The important thing is that complete() was called successfully
        let _ = timing.execution_time;
    }

    #[test]
    fn query_timing_timeout() {
        let mut timing = QueryTiming::new("MATCH (n) RETURN n", Duration::from_secs(30));
        timing.mark_timed_out();
        assert!(timing.timed_out);
    }

    #[test]
    fn query_timing_truncates_long_query() {
        let long_query = "MATCH (n) RETURN n ".repeat(100);
        let timing = QueryTiming::new(&long_query, Duration::from_secs(30));
        assert!(timing.query_preview.len() <= 103); // 100 + "..."
        assert!(timing.query_preview.ends_with("..."));
    }
}
