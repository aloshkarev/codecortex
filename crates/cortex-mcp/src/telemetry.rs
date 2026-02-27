//! Telemetry and Tracing for CodeCortex MCP Tools
//!
//! Provides structured telemetry collection for observability including:
//! - Tool call duration tracking
//! - Cache hit/miss ratios
//! - Partial response tracking
//! - Timeout guard events
//! - Query performance metrics

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Telemetry data for a single tool invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolTelemetry {
    /// Name of the tool that was invoked
    pub tool_name: String,
    /// Duration of the tool call in milliseconds
    pub duration_ms: u64,
    /// Cache hit level: "l1" | "l2" | "none"
    pub cache_hit: String,
    /// Whether a partial response was returned
    pub partial_response: bool,
    /// Whether a timeout guard was triggered
    pub timeout_guard_triggered: bool,
    /// Number of rows/records scanned (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_scanned: Option<usize>,
    /// Unique request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Timestamp when the telemetry was recorded
    pub timestamp_ms: u64,
    /// Repository path for the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
    /// Additional metrics specific to the tool
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra_metrics: HashMap<String, serde_json::Value>,
}

impl ToolTelemetry {
    /// Create a new telemetry record
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            duration_ms: 0,
            cache_hit: "none".to_string(),
            partial_response: false,
            timeout_guard_triggered: false,
            rows_scanned: None,
            request_id: None,
            timestamp_ms: current_timestamp_ms(),
            repo_path: None,
            extra_metrics: HashMap::new(),
        }
    }

    /// Set the duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Set the cache hit level
    pub fn with_cache_hit(mut self, cache_hit: &str) -> Self {
        self.cache_hit = cache_hit.to_string();
        self
    }

    /// Set partial response flag
    pub fn with_partial(mut self, partial: bool) -> Self {
        self.partial_response = partial;
        self
    }

    /// Set timeout guard triggered flag
    pub fn with_timeout_guard(mut self, triggered: bool) -> Self {
        self.timeout_guard_triggered = triggered;
        self
    }

    /// Set rows scanned count
    pub fn with_rows_scanned(mut self, count: usize) -> Self {
        self.rows_scanned = Some(count);
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Set repository path
    pub fn with_repo_path(mut self, path: impl Into<String>) -> Self {
        self.repo_path = Some(path.into());
        self
    }

    /// Add an extra metric
    pub fn with_metric(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra_metrics.insert(key.into(), value);
        self
    }
}

/// Builder for collecting telemetry during a tool invocation
pub struct TelemetryCollector {
    tool_name: String,
    started_at: Instant,
    request_id: Option<String>,
    repo_path: Option<String>,
    cache_hit: String,
    rows_scanned: Option<usize>,
    extra_metrics: HashMap<String, serde_json::Value>,
}

impl TelemetryCollector {
    /// Start collecting telemetry for a tool invocation
    pub fn start(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            started_at: Instant::now(),
            request_id: None,
            repo_path: None,
            cache_hit: "none".to_string(),
            rows_scanned: None,
            extra_metrics: HashMap::new(),
        }
    }

    /// Set the request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    /// Set the repository path
    pub fn with_repo_path(mut self, path: impl Into<String>) -> Self {
        self.repo_path = Some(path.into());
        self
    }

    /// Record a cache hit
    pub fn record_cache_hit(&mut self, level: &str) {
        self.cache_hit = level.to_string();
    }

    /// Record rows scanned
    pub fn record_rows_scanned(&mut self, count: usize) {
        self.rows_scanned = Some(count);
    }

    /// Add an extra metric
    pub fn add_metric(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.extra_metrics.insert(key.into(), value);
    }

    /// Finish collection and create telemetry record
    pub fn finish(self, partial: bool, timeout_triggered: bool) -> ToolTelemetry {
        ToolTelemetry {
            tool_name: self.tool_name,
            duration_ms: self.started_at.elapsed().as_millis() as u64,
            cache_hit: self.cache_hit,
            partial_response: partial,
            timeout_guard_triggered: timeout_triggered,
            rows_scanned: self.rows_scanned,
            request_id: self.request_id,
            timestamp_ms: current_timestamp_ms(),
            repo_path: self.repo_path,
            extra_metrics: self.extra_metrics,
        }
    }
}

/// Aggregated statistics for a tool
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolStats {
    /// Total number of invocations
    pub total_calls: u64,
    /// Number of successful calls
    pub successful_calls: u64,
    /// Number of partial responses
    pub partial_responses: u64,
    /// Number of timeout events
    pub timeouts: u64,
    /// Cache hits by level
    pub cache_hits: HashMap<String, u64>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Minimum duration in milliseconds
    pub min_duration_ms: Option<u64>,
    /// Maximum duration in milliseconds
    pub max_duration_ms: Option<u64>,
    /// Total rows scanned
    pub total_rows_scanned: u64,
}

impl ToolStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self {
            total_calls: 0,
            successful_calls: 0,
            partial_responses: 0,
            timeouts: 0,
            cache_hits: HashMap::new(),
            total_duration_ms: 0,
            min_duration_ms: None,
            max_duration_ms: None,
            total_rows_scanned: 0,
        }
    }

    /// Record a telemetry event
    pub fn record(&mut self, telemetry: &ToolTelemetry) {
        self.total_calls += 1;

        if !telemetry.partial_response {
            self.successful_calls += 1;
        } else {
            self.partial_responses += 1;
        }

        if telemetry.timeout_guard_triggered {
            self.timeouts += 1;
        }

        *self
            .cache_hits
            .entry(telemetry.cache_hit.clone())
            .or_insert(0) += 1;

        self.total_duration_ms += telemetry.duration_ms;

        self.min_duration_ms = Some(
            self.min_duration_ms
                .map_or(telemetry.duration_ms, |m| m.min(telemetry.duration_ms)),
        );

        self.max_duration_ms = Some(
            self.max_duration_ms
                .map_or(telemetry.duration_ms, |m| m.max(telemetry.duration_ms)),
        );

        if let Some(rows) = telemetry.rows_scanned {
            self.total_rows_scanned += rows as u64;
        }
    }

    /// Get average duration in milliseconds
    pub fn avg_duration_ms(&self) -> f64 {
        if self.total_calls == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.total_calls as f64
        }
    }

    /// Get p50 (median) duration estimate (simplified)
    pub fn p50_estimate_ms(&self) -> f64 {
        // Simple estimate: use average as proxy for median
        self.avg_duration_ms()
    }

    /// Get p95 duration estimate (simplified)
    pub fn p95_estimate_ms(&self) -> f64 {
        // Simple estimate: max * 0.8 or avg * 1.5
        self.max_duration_ms
            .map(|m| (m as f64 * 0.8).max(self.avg_duration_ms() * 1.5))
            .unwrap_or(0.0)
    }

    /// Get cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let hits: u64 = self
            .cache_hits
            .iter()
            .filter(|(k, _)| *k != "none")
            .map(|(_, v)| *v)
            .sum();

        if self.total_calls == 0 {
            0.0
        } else {
            hits as f64 / self.total_calls as f64
        }
    }
}

/// Global telemetry registry
#[derive(Debug, Default)]
pub struct TelemetryRegistry {
    /// Statistics by tool name
    stats: HashMap<String, ToolStats>,
    /// Recent telemetry records (ring buffer)
    recent: Vec<ToolTelemetry>,
    /// Maximum recent records to keep
    max_recent: usize,
}

impl TelemetryRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            recent: Vec::new(),
            max_recent: 1000,
        }
    }

    /// Create a new registry with custom max recent records
    pub fn with_max_recent(max_recent: usize) -> Self {
        Self {
            stats: HashMap::new(),
            recent: Vec::new(),
            max_recent,
        }
    }

    /// Record a telemetry event
    pub fn record(&mut self, telemetry: ToolTelemetry) {
        // Update stats
        self.stats
            .entry(telemetry.tool_name.clone())
            .or_default()
            .record(&telemetry);

        // Add to recent records
        self.recent.push(telemetry);

        // Trim if needed
        if self.recent.len() > self.max_recent {
            self.recent.remove(0);
        }
    }

    /// Get stats for a specific tool
    pub fn get_stats(&self, tool_name: &str) -> Option<&ToolStats> {
        self.stats.get(tool_name)
    }

    /// Get all stats
    pub fn all_stats(&self) -> &HashMap<String, ToolStats> {
        &self.stats
    }

    /// Get recent telemetry records
    pub fn recent(&self) -> &[ToolTelemetry] {
        &self.recent
    }

    /// Get recent telemetry for a specific tool
    pub fn recent_for_tool(&self, tool_name: &str) -> Vec<&ToolTelemetry> {
        self.recent
            .iter()
            .filter(|t| t.tool_name == tool_name)
            .collect()
    }

    /// Clear all statistics and recent records
    pub fn clear(&mut self) {
        self.stats.clear();
        self.recent.clear();
    }

    /// Get a summary of all tool statistics
    pub fn summary(&self) -> TelemetrySummary {
        let mut summary = TelemetrySummary {
            total_tools: self.stats.len(),
            total_calls: 0,
            avg_duration_ms: 0.0,
            overall_cache_hit_ratio: 0.0,
            tool_summaries: HashMap::new(),
        };

        let mut total_duration = 0u64;
        let mut total_hits = 0u64;

        for (name, stats) in &self.stats {
            summary.total_calls += stats.total_calls;
            total_duration += stats.total_duration_ms;

            let hits: u64 = stats
                .cache_hits
                .iter()
                .filter(|(k, _)| *k != "none")
                .map(|(_, v)| *v)
                .sum();
            total_hits += hits;

            summary.tool_summaries.insert(
                name.clone(),
                ToolSummary {
                    total_calls: stats.total_calls,
                    avg_duration_ms: stats.avg_duration_ms(),
                    cache_hit_ratio: stats.cache_hit_ratio(),
                },
            );
        }

        if summary.total_calls > 0 {
            summary.avg_duration_ms = total_duration as f64 / summary.total_calls as f64;
            summary.overall_cache_hit_ratio = total_hits as f64 / summary.total_calls as f64;
        }

        summary
    }
}

/// Summary of telemetry data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySummary {
    /// Total number of tools with recorded telemetry
    pub total_tools: usize,
    /// Total number of calls across all tools
    pub total_calls: u64,
    /// Average duration across all calls
    pub avg_duration_ms: f64,
    /// Overall cache hit ratio
    pub overall_cache_hit_ratio: f64,
    /// Per-tool summaries
    pub tool_summaries: HashMap<String, ToolSummary>,
}

/// Summary for a single tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSummary {
    /// Total calls for this tool
    pub total_calls: u64,
    /// Average duration in milliseconds
    pub avg_duration_ms: f64,
    /// Cache hit ratio for this tool
    pub cache_hit_ratio: f64,
}

/// Get current timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Global telemetry registry (lazy initialized)
static TELEMETRY_REGISTRY: std::sync::OnceLock<std::sync::Mutex<TelemetryRegistry>> =
    std::sync::OnceLock::new();

/// Get the global telemetry registry
pub fn global_registry() -> &'static std::sync::Mutex<TelemetryRegistry> {
    TELEMETRY_REGISTRY.get_or_init(|| std::sync::Mutex::new(TelemetryRegistry::new()))
}

/// Record telemetry to the global registry
pub fn record_telemetry(telemetry: ToolTelemetry) {
    if let Ok(mut registry) = global_registry().lock() {
        registry.record(telemetry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_telemetry_new() {
        let t = ToolTelemetry::new("test_tool");
        assert_eq!(t.tool_name, "test_tool");
        assert_eq!(t.duration_ms, 0);
        assert_eq!(t.cache_hit, "none");
        assert!(!t.partial_response);
        assert!(!t.timeout_guard_triggered);
    }

    #[test]
    fn tool_telemetry_builder() {
        let t = ToolTelemetry::new("test_tool")
            .with_duration(100)
            .with_cache_hit("l1")
            .with_partial(true)
            .with_timeout_guard(true)
            .with_rows_scanned(500)
            .with_request_id("req-123")
            .with_repo_path("/repo/path");

        assert_eq!(t.duration_ms, 100);
        assert_eq!(t.cache_hit, "l1");
        assert!(t.partial_response);
        assert!(t.timeout_guard_triggered);
        assert_eq!(t.rows_scanned, Some(500));
        assert_eq!(t.request_id, Some("req-123".to_string()));
        assert_eq!(t.repo_path, Some("/repo/path".to_string()));
    }

    #[test]
    fn telemetry_collector() {
        let mut collector = TelemetryCollector::start("test_tool")
            .with_request_id("req-123")
            .with_repo_path("/repo");

        collector.record_cache_hit("l2");
        collector.record_rows_scanned(100);
        collector.add_metric("custom", serde_json::json!(42));

        let t = collector.finish(false, false);

        assert_eq!(t.tool_name, "test_tool");
        assert!(t.duration_ms < 1000); // Should be fast
        assert_eq!(t.cache_hit, "l2");
        assert_eq!(t.rows_scanned, Some(100));
        assert_eq!(t.extra_metrics.get("custom"), Some(&serde_json::json!(42)));
    }

    #[test]
    fn tool_stats() {
        let mut stats = ToolStats::new();

        stats.record(&ToolTelemetry::new("tool1").with_duration(100));
        stats.record(&ToolTelemetry::new("tool1").with_duration(200));
        stats.record(
            &ToolTelemetry::new("tool1")
                .with_duration(300)
                .with_cache_hit("l1"),
        );

        assert_eq!(stats.total_calls, 3);
        assert_eq!(stats.successful_calls, 3);
        assert_eq!(stats.total_duration_ms, 600);
        assert_eq!(stats.min_duration_ms, Some(100));
        assert_eq!(stats.max_duration_ms, Some(300));
        assert_eq!(stats.cache_hits.get("none"), Some(&2));
        assert_eq!(stats.cache_hits.get("l1"), Some(&1));
    }

    #[test]
    fn tool_stats_ratios() {
        let mut stats = ToolStats::new();

        stats.record(&ToolTelemetry::new("tool").with_cache_hit("l1"));
        stats.record(&ToolTelemetry::new("tool").with_cache_hit("l2"));
        stats.record(&ToolTelemetry::new("tool").with_cache_hit("none"));

        assert!((stats.cache_hit_ratio() - 0.6666666666666666).abs() < 0.01);
    }

    #[test]
    fn telemetry_registry() {
        let mut registry = TelemetryRegistry::new();

        registry.record(ToolTelemetry::new("tool1").with_duration(100));
        registry.record(ToolTelemetry::new("tool1").with_duration(200));
        registry.record(ToolTelemetry::new("tool2").with_duration(300));

        let stats1 = registry.get_stats("tool1").unwrap();
        assert_eq!(stats1.total_calls, 2);

        let stats2 = registry.get_stats("tool2").unwrap();
        assert_eq!(stats2.total_calls, 1);

        assert_eq!(registry.recent().len(), 3);
    }

    #[test]
    fn telemetry_summary() {
        let mut registry = TelemetryRegistry::new();

        registry.record(ToolTelemetry::new("tool1").with_duration(100));
        registry.record(
            ToolTelemetry::new("tool1")
                .with_duration(200)
                .with_cache_hit("l1"),
        );
        registry.record(
            ToolTelemetry::new("tool2")
                .with_duration(300)
                .with_cache_hit("l2"),
        );

        let summary = registry.summary();

        assert_eq!(summary.total_tools, 2);
        assert_eq!(summary.total_calls, 3);
        assert!((summary.avg_duration_ms - 200.0).abs() < 0.01);
        assert!((summary.overall_cache_hit_ratio - 0.6666666666666666).abs() < 0.01);
    }
}
