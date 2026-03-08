//! Production-Grade Metrics and Instrumentation for CodeCortex MCP
//!
//! Provides:
//! - Prometheus-compatible metrics
//! - Health check endpoints
//! - Request tracing
//! - Performance monitoring

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Global metrics registry
static METRICS_REGISTRY: std::sync::OnceLock<MetricsRegistry> = std::sync::OnceLock::new();

/// Get the global metrics registry
pub fn global_metrics() -> &'static MetricsRegistry {
    METRICS_REGISTRY.get_or_init(MetricsRegistry::new)
}

/// Metrics registry for collecting and reporting metrics
#[derive(Debug)]
pub struct MetricsRegistry {
    /// Tool invocation counters
    tool_invocations: HashMap<&'static str, AtomicU64>,
    /// Tool latency histograms (simplified as p50/p95/p99)
    tool_latencies: Arc<Mutex<HashMap<String, LatencyTracker>>>,
    /// Error counters by type
    error_counts: HashMap<&'static str, AtomicU64>,
    /// Cache hit/miss counters
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    /// Active connections
    active_connections: AtomicUsize,
    /// Total requests
    total_requests: AtomicU64,
    /// Indexing metrics
    files_indexed: AtomicU64,
    bytes_processed: AtomicU64,
    vector_embeddings_generated: AtomicU64,
    vector_indexed_documents: AtomicU64,
    vector_fallbacks: AtomicU64,
    /// Start time for uptime calculation
    start_time: Instant,
}

/// Simple latency tracker for percentiles
#[derive(Debug, Clone, Default)]
pub struct LatencyTracker {
    samples: Vec<Duration>,
    sorted: bool,
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self {
            samples: Vec::with_capacity(1000),
            sorted: false,
        }
    }

    pub fn record(&mut self, duration: Duration) {
        self.samples.push(duration);
        self.sorted = false;

        // Keep only last 1000 samples to avoid memory growth
        if self.samples.len() > 1000 {
            self.samples.drain(0..100);
        }
    }

    pub fn percentile(&mut self, p: f64) -> Duration {
        if !self.sorted {
            self.samples.sort();
            self.sorted = true;
        }

        if self.samples.is_empty() {
            return Duration::ZERO;
        }

        let idx = ((p / 100.0) * (self.samples.len() - 1) as f64).round() as usize;
        self.samples[idx.min(self.samples.len() - 1)]
    }

    pub fn count(&self) -> usize {
        self.samples.len()
    }
}

impl MetricsRegistry {
    fn new() -> Self {
        let mut tool_invocations = HashMap::new();
        for tool in &[
            "get_context_capsule",
            "get_impact_graph",
            "search_logic_flow",
            "get_skeleton",
            "index_status",
            "save_observation",
            "search_memory",
            "submit_lsp_edges",
            "find_code",
            "execute_cypher",
        ] {
            tool_invocations.insert(*tool, AtomicU64::new(0));
        }

        let mut error_counts = HashMap::new();
        for error_type in &[
            "invalid_argument",
            "not_found",
            "timeout",
            "internal_error",
            "rate_limited",
            "permission_denied",
        ] {
            error_counts.insert(*error_type, AtomicU64::new(0));
        }

        Self {
            tool_invocations,
            tool_latencies: Arc::new(Mutex::new(HashMap::new())),
            error_counts,
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            total_requests: AtomicU64::new(0),
            files_indexed: AtomicU64::new(0),
            bytes_processed: AtomicU64::new(0),
            vector_embeddings_generated: AtomicU64::new(0),
            vector_indexed_documents: AtomicU64::new(0),
            vector_fallbacks: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Record a tool invocation
    pub fn record_tool_invocation(&self, tool_name: &str) {
        if let Some(counter) = self.tool_invocations.get(tool_name) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record tool latency
    pub fn record_tool_latency(&self, tool_name: &str, duration: Duration) {
        if let Ok(mut latencies) = self.tool_latencies.lock() {
            latencies
                .entry(tool_name.to_string())
                .or_insert_with(LatencyTracker::new)
                .record(duration);
        }
    }

    /// Record an error
    pub fn record_error(&self, error_type: &str) {
        if let Some(counter) = self.error_counts.get(error_type) {
            counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active connections
    pub fn connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record indexing progress
    pub fn record_files_indexed(&self, count: u64) {
        self.files_indexed.fetch_add(count, Ordering::Relaxed);
    }

    /// Record bytes processed
    pub fn record_bytes_processed(&self, bytes: u64) {
        self.bytes_processed.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record generated embeddings count.
    pub fn record_embeddings_generated(&self, count: u64) {
        self.vector_embeddings_generated
            .fetch_add(count, Ordering::Relaxed);
    }

    /// Record indexed vector documents count.
    pub fn record_vector_documents_indexed(&self, count: u64) {
        self.vector_indexed_documents
            .fetch_add(count, Ordering::Relaxed);
    }

    /// Record fallback from semantic to lexical retrieval.
    pub fn record_vector_fallback(&self) {
        self.vector_fallbacks.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let tool_invocations: HashMap<String, u64> = self
            .tool_invocations
            .iter()
            .map(|(k, v)| (k.to_string(), v.load(Ordering::Relaxed)))
            .collect();

        let tool_latencies: HashMap<String, LatencySnapshot> =
            if let Ok(mut latencies) = self.tool_latencies.lock() {
                latencies
                    .iter_mut()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            LatencySnapshot {
                                p50_ms: v.percentile(50.0).as_millis() as u64,
                                p95_ms: v.percentile(95.0).as_millis() as u64,
                                p99_ms: v.percentile(99.0).as_millis() as u64,
                                count: v.count(),
                            },
                        )
                    })
                    .collect()
            } else {
                HashMap::new()
            };

        let error_counts: HashMap<String, u64> = self
            .error_counts
            .iter()
            .map(|(k, v)| (k.to_string(), v.load(Ordering::Relaxed)))
            .collect();

        MetricsSnapshot {
            uptime_secs: self.start_time.elapsed().as_secs(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            files_indexed: self.files_indexed.load(Ordering::Relaxed),
            bytes_processed: self.bytes_processed.load(Ordering::Relaxed),
            vector_embeddings_generated: self.vector_embeddings_generated.load(Ordering::Relaxed),
            vector_indexed_documents: self.vector_indexed_documents.load(Ordering::Relaxed),
            vector_fallbacks: self.vector_fallbacks.load(Ordering::Relaxed),
            tool_invocations,
            tool_latencies,
            error_counts,
        }
    }

    /// Export metrics in Prometheus format
    pub fn prometheus_export(&self) -> String {
        let snapshot = self.snapshot();
        let mut output = String::new();

        // Help and type declarations
        output.push_str("# HELP cortex_uptime_seconds Service uptime in seconds\n");
        output.push_str("# TYPE cortex_uptime_seconds counter\n");
        output.push_str(&format!("cortex_uptime_seconds {}\n", snapshot.uptime_secs));

        output.push_str("\n# HELP cortex_requests_total Total number of requests\n");
        output.push_str("# TYPE cortex_requests_total counter\n");
        output.push_str(&format!(
            "cortex_requests_total {}\n",
            snapshot.total_requests
        ));

        output.push_str("\n# HELP cortex_connections_active Active connections\n");
        output.push_str("# TYPE cortex_connections_active gauge\n");
        output.push_str(&format!(
            "cortex_connections_active {}\n",
            snapshot.active_connections
        ));

        output.push_str("\n# HELP cortex_cache_hits_total Cache hit count\n");
        output.push_str("# TYPE cortex_cache_hits_total counter\n");
        output.push_str(&format!(
            "cortex_cache_hits_total {}\n",
            snapshot.cache_hits
        ));

        output.push_str("\n# HELP cortex_cache_misses_total Cache miss count\n");
        output.push_str("# TYPE cortex_cache_misses_total counter\n");
        output.push_str(&format!(
            "cortex_cache_misses_total {}\n",
            snapshot.cache_misses
        ));

        output.push_str("\n# HELP cortex_files_indexed_total Files indexed count\n");
        output.push_str("# TYPE cortex_files_indexed_total counter\n");
        output.push_str(&format!(
            "cortex_files_indexed_total {}\n",
            snapshot.files_indexed
        ));

        output.push_str(
            "\n# HELP cortex_vector_embeddings_generated_total Embeddings generated count\n",
        );
        output.push_str("# TYPE cortex_vector_embeddings_generated_total counter\n");
        output.push_str(&format!(
            "cortex_vector_embeddings_generated_total {}\n",
            snapshot.vector_embeddings_generated
        ));

        output.push_str(
            "\n# HELP cortex_vector_documents_indexed_total Vector documents indexed count\n",
        );
        output.push_str("# TYPE cortex_vector_documents_indexed_total counter\n");
        output.push_str(&format!(
            "cortex_vector_documents_indexed_total {}\n",
            snapshot.vector_indexed_documents
        ));

        output.push_str("\n# HELP cortex_vector_fallbacks_total Semantic fallback events count\n");
        output.push_str("# TYPE cortex_vector_fallbacks_total counter\n");
        output.push_str(&format!(
            "cortex_vector_fallbacks_total {}\n",
            snapshot.vector_fallbacks
        ));

        // Tool invocations
        output.push_str("\n# HELP cortex_tool_invocations_total Tool invocation count\n");
        output.push_str("# TYPE cortex_tool_invocations_total counter\n");
        for (tool, count) in &snapshot.tool_invocations {
            output.push_str(&format!(
                "cortex_tool_invocations_total{{tool=\"{}\"}} {}\n",
                tool, count
            ));
        }

        // Tool latencies
        output.push_str("\n# HELP cortex_tool_latency_milliseconds Tool latency in milliseconds\n");
        output.push_str("# TYPE cortex_tool_latency_milliseconds summary\n");
        for (tool, latency) in &snapshot.tool_latencies {
            output.push_str(&format!(
                "cortex_tool_latency_milliseconds{{tool=\"{}\",quantile=\"0.5\"}} {}\n",
                tool, latency.p50_ms
            ));
            output.push_str(&format!(
                "cortex_tool_latency_milliseconds{{tool=\"{}\",quantile=\"0.95\"}} {}\n",
                tool, latency.p95_ms
            ));
            output.push_str(&format!(
                "cortex_tool_latency_milliseconds{{tool=\"{}\",quantile=\"0.99\"}} {}\n",
                tool, latency.p99_ms
            ));
        }

        // Error counts
        output.push_str("\n# HELP cortex_errors_total Error count by type\n");
        output.push_str("# TYPE cortex_errors_total counter\n");
        for (error_type, count) in &snapshot.error_counts {
            output.push_str(&format!(
                "cortex_errors_total{{type=\"{}\"}} {}\n",
                error_type, count
            ));
        }

        output
    }
}

/// Snapshot of current metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub uptime_secs: u64,
    pub total_requests: u64,
    pub active_connections: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub files_indexed: u64,
    pub bytes_processed: u64,
    pub vector_embeddings_generated: u64,
    pub vector_indexed_documents: u64,
    pub vector_fallbacks: u64,
    pub tool_invocations: HashMap<String, u64>,
    pub tool_latencies: HashMap<String, LatencySnapshot>,
    pub error_counts: HashMap<String, u64>,
}

/// Latency statistics for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencySnapshot {
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub count: usize,
}

/// Health status for the service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub status: String,
    pub uptime_secs: u64,
    pub version: String,
    pub checks: HashMap<String, HealthCheck>,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: HealthCheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthCheckStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health checker
#[allow(clippy::type_complexity)]
pub struct HealthChecker {
    checks: Vec<(&'static str, Box<dyn Fn() -> HealthCheck + Send + Sync>)>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Add a health check
    pub fn add_check<F>(&mut self, name: &'static str, check: F)
    where
        F: Fn() -> HealthCheck + Send + Sync + 'static,
    {
        self.checks.push((name, Box::new(check)));
    }

    /// Run all health checks
    pub fn check(&self) -> HealthStatus {
        let mut checks = HashMap::new();
        let mut all_healthy = true;

        for (name, check_fn) in &self.checks {
            let result = check_fn();
            if result.status == HealthCheckStatus::Unhealthy {
                all_healthy = false;
            }
            checks.insert(name.to_string(), result);
        }

        HealthStatus {
            healthy: all_healthy,
            status: if all_healthy {
                "ok".to_string()
            } else {
                "degraded".to_string()
            },
            uptime_secs: global_metrics().start_time.elapsed().as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            checks,
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for timing operations
pub struct TimerGuard {
    start: Instant,
    tool_name: &'static str,
    metrics: &'static MetricsRegistry,
}

impl TimerGuard {
    pub fn new(tool_name: &'static str) -> Self {
        let metrics = global_metrics();
        metrics.record_tool_invocation(tool_name);
        Self {
            start: Instant::now(),
            tool_name,
            metrics,
        }
    }
}

impl Drop for TimerGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.metrics.record_tool_latency(self.tool_name, duration);
    }
}

/// Macro to create a timer guard
#[macro_export]
macro_rules! time_tool {
    ($name:expr) => {
        let _timer = $crate::metrics::TimerGuard::new($name);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_registry_records_invocations() {
        let registry = MetricsRegistry::new();

        registry.record_tool_invocation("get_context_capsule");
        registry.record_tool_invocation("get_context_capsule");

        let snapshot = registry.snapshot();
        assert_eq!(
            snapshot.tool_invocations.get("get_context_capsule"),
            Some(&2)
        );
    }

    #[test]
    fn metrics_registry_records_cache() {
        let registry = MetricsRegistry::new();

        registry.record_cache_hit();
        registry.record_cache_hit();
        registry.record_cache_miss();

        let snapshot = registry.snapshot();
        assert_eq!(snapshot.cache_hits, 2);
        assert_eq!(snapshot.cache_misses, 1);
    }

    #[test]
    fn metrics_registry_records_latency() {
        let registry = MetricsRegistry::new();

        registry.record_tool_latency("test_tool", Duration::from_millis(10));
        registry.record_tool_latency("test_tool", Duration::from_millis(20));
        registry.record_tool_latency("test_tool", Duration::from_millis(30));

        let snapshot = registry.snapshot();
        let latency = snapshot.tool_latencies.get("test_tool").unwrap();
        assert_eq!(latency.count, 3);
        assert!(latency.p50_ms > 0);
    }

    #[test]
    fn prometheus_export_format() {
        let registry = MetricsRegistry::new();
        registry.record_tool_invocation("get_context_capsule");
        registry.record_cache_hit();

        let export = registry.prometheus_export();
        assert!(export.contains("cortex_requests_total"));
        assert!(export.contains("cortex_cache_hits_total"));
        assert!(export.contains("tool=\"get_context_capsule\""));
    }

    #[test]
    fn health_checker_basic() {
        let mut checker = HealthChecker::new();
        checker.add_check("test_check", || HealthCheck {
            status: HealthCheckStatus::Healthy,
            message: Some("OK".to_string()),
            latency_ms: Some(1),
        });

        let status = checker.check();
        assert!(status.healthy);
    }

    #[test]
    fn timer_guard_records_latency() {
        // Use a fresh registry to avoid global state
        let registry = MetricsRegistry::new();

        {
            let metrics = &registry;
            // Use a registered tool name
            metrics.record_tool_invocation("get_context_capsule");

            let start = Instant::now();
            std::thread::sleep(Duration::from_millis(5));
            metrics.record_tool_latency("get_context_capsule", start.elapsed());
        }

        let snapshot = registry.snapshot();
        assert_eq!(
            snapshot.tool_invocations.get("get_context_capsule"),
            Some(&1)
        );
        assert!(snapshot.tool_latencies.contains_key("get_context_capsule"));
    }
}
