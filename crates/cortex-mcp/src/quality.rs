//! Quality Metrics for MCP Tools
//!
//! Provides comprehensive quality tracking:
//! - Tool reliability scoring
//! - Error rate monitoring
//! - Latency percentiles
//! - Health scoring
//! - Quality trends

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Quality metrics for a single tool
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolQualityMetrics {
    /// Tool name
    pub tool_name: String,
    /// Total invocations
    pub total_invocations: u64,
    /// Successful invocations
    pub successful_invocations: u64,
    /// Failed invocations
    pub failed_invocations: u64,
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Average latency in ms
    pub avg_latency_ms: f64,
    /// P50 latency in ms
    pub p50_latency_ms: u64,
    /// P95 latency in ms
    pub p95_latency_ms: u64,
    /// P99 latency in ms
    pub p99_latency_ms: u64,
    /// Quality score (0.0 - 1.0)
    pub quality_score: f64,
    /// Reliability score (0.0 - 1.0)
    pub reliability_score: f64,
    /// Performance score (0.0 - 1.0)
    pub performance_score: f64,
    /// Last invocation time
    pub last_invocation: Option<u64>,
    /// Time since last error
    pub last_error_secs: Option<u64>,
}

/// Quality configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    /// Target error rate (for scoring)
    pub target_error_rate: f64,
    /// Target P95 latency in ms
    pub target_p95_latency_ms: u64,
    /// Weight for reliability in quality score
    pub reliability_weight: f64,
    /// Weight for performance in quality score
    pub performance_weight: f64,
    /// Number of samples for rolling calculations
    pub sample_window: usize,
    /// Latency threshold for "slow" classification
    pub slow_threshold_ms: u64,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            target_error_rate: 0.01,     // 1% target
            target_p95_latency_ms: 1000, // 1s target
            reliability_weight: 0.6,
            performance_weight: 0.4,
            sample_window: 100,
            slow_threshold_ms: 2000,
        }
    }
}

/// Quality tracker for a single tool
#[derive(Debug)]
struct ToolTracker {
    total: AtomicU64,
    successful: AtomicU64,
    failed: AtomicU64,
    latencies: Mutex<Vec<Duration>>,
    errors: Mutex<Vec<Instant>>,
    last_invocation: Mutex<Option<Instant>>,
    config: QualityConfig,
}

impl ToolTracker {
    fn new(config: QualityConfig) -> Self {
        Self {
            total: AtomicU64::new(0),
            successful: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            latencies: Mutex::new(Vec::with_capacity(config.sample_window)),
            errors: Mutex::new(Vec::new()),
            last_invocation: Mutex::new(None),
            config,
        }
    }

    fn record_success(&self, latency: Duration) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.successful.fetch_add(1, Ordering::Relaxed);

        if let Ok(mut latencies) = self.latencies.lock() {
            latencies.push(latency);
            if latencies.len() > self.config.sample_window {
                latencies.remove(0);
            }
        }

        if let Ok(mut last) = self.last_invocation.lock() {
            *last = Some(Instant::now());
        }
    }

    fn record_failure(&self, latency: Duration) {
        self.total.fetch_add(1, Ordering::Relaxed);
        self.failed.fetch_add(1, Ordering::Relaxed);

        if let Ok(mut errors) = self.errors.lock() {
            errors.push(Instant::now());
            if errors.len() > self.config.sample_window {
                errors.remove(0);
            }
        }

        if let Ok(mut latencies) = self.latencies.lock() {
            latencies.push(latency);
            if latencies.len() > self.config.sample_window {
                latencies.remove(0);
            }
        }

        if let Ok(mut last) = self.last_invocation.lock() {
            *last = Some(Instant::now());
        }
    }

    fn calculate_metrics(&self, tool_name: &str) -> ToolQualityMetrics {
        let total = self.total.load(Ordering::Relaxed);
        let successful = self.successful.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);

        let error_rate = if total > 0 {
            failed as f64 / total as f64
        } else {
            0.0
        };

        let (avg_latency_ms, p50, p95, p99) = if let Ok(latencies) = self.latencies.lock() {
            if latencies.is_empty() {
                (0.0, 0, 0, 0)
            } else {
                let mut sorted: Vec<_> = latencies.clone();
                sorted.sort();

                let sum: Duration = sorted.iter().sum();
                let avg = sum.as_millis() as f64 / sorted.len() as f64;

                let p50_idx = (sorted.len() as f64 * 0.50) as usize;
                let p95_idx = (sorted.len() as f64 * 0.95) as usize;
                let p99_idx = (sorted.len() as f64 * 0.99) as usize;

                (
                    avg,
                    sorted
                        .get(p50_idx)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    sorted
                        .get(p95_idx.min(sorted.len() - 1))
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                    sorted
                        .get(p99_idx.min(sorted.len() - 1))
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                )
            }
        } else {
            (0.0, 0, 0, 0)
        };

        // Calculate reliability score (0-1, higher is better)
        let reliability_score = 1.0 - (error_rate / self.config.target_error_rate).min(1.0);

        // Calculate performance score (0-1, higher is better)
        let performance_score = if p95 == 0 {
            1.0
        } else {
            1.0 - (p95 as f64 / self.config.target_p95_latency_ms as f64 / 2.0).min(1.0)
        };

        // Combined quality score
        let quality_score = (reliability_score * self.config.reliability_weight
            + performance_score * self.config.performance_weight)
            .min(1.0);

        let last_invocation = if let Ok(last) = self.last_invocation.lock() {
            last.map(|t| {
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    - t.elapsed().as_secs()
            })
        } else {
            None
        };

        let last_error_secs = if let Ok(errors) = self.errors.lock() {
            errors.last().map(|t| t.elapsed().as_secs())
        } else {
            None
        };

        ToolQualityMetrics {
            tool_name: tool_name.to_string(),
            total_invocations: total,
            successful_invocations: successful,
            failed_invocations: failed,
            error_rate,
            avg_latency_ms,
            p50_latency_ms: p50,
            p95_latency_ms: p95,
            p99_latency_ms: p99,
            quality_score,
            reliability_score,
            performance_score,
            last_invocation,
            last_error_secs,
        }
    }
}

use std::time::SystemTime;

/// Quality registry for all tools
pub struct QualityRegistry {
    tools: Mutex<HashMap<String, Arc<ToolTracker>>>,
    config: QualityConfig,
    start_time: Instant,
}

impl QualityRegistry {
    /// Create a new quality registry
    pub fn new(config: QualityConfig) -> Self {
        Self {
            tools: Mutex::new(HashMap::new()),
            config,
            start_time: Instant::now(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(QualityConfig::default())
    }

    /// Get or create a tracker for a tool
    fn get_tracker(&self, tool_name: &str) -> Arc<ToolTracker> {
        let mut tools = self.tools.lock().unwrap();
        tools
            .entry(tool_name.to_string())
            .or_insert_with(|| Arc::new(ToolTracker::new(self.config.clone())))
            .clone()
    }

    /// Record a successful invocation
    pub fn record_success(&self, tool_name: &str, latency: Duration) {
        let tracker = self.get_tracker(tool_name);
        tracker.record_success(latency);
    }

    /// Record a failed invocation
    pub fn record_failure(&self, tool_name: &str, latency: Duration) {
        let tracker = self.get_tracker(tool_name);
        tracker.record_failure(latency);
    }

    /// Get metrics for a specific tool
    pub fn get_metrics(&self, tool_name: &str) -> Option<ToolQualityMetrics> {
        let tools = self.tools.lock().unwrap();
        tools.get(tool_name).map(|t| t.calculate_metrics(tool_name))
    }

    /// Get metrics for all tools
    pub fn get_all_metrics(&self) -> Vec<ToolQualityMetrics> {
        let tools = self.tools.lock().unwrap();
        tools
            .iter()
            .map(|(name, tracker)| tracker.calculate_metrics(name))
            .collect()
    }

    /// Get overall system quality score
    pub fn system_quality_score(&self) -> f64 {
        let metrics = self.get_all_metrics();
        if metrics.is_empty() {
            return 1.0;
        }

        let total_invocations: u64 = metrics.iter().map(|m| m.total_invocations).sum();
        if total_invocations == 0 {
            return 1.0;
        }

        // Weighted average by invocation count
        let weighted_score: f64 = metrics
            .iter()
            .map(|m| m.quality_score * m.total_invocations as f64)
            .sum();

        weighted_score / total_invocations as f64
    }

    /// Get health status
    pub fn health_status(&self) -> QualityHealthStatus {
        let metrics = self.get_all_metrics();
        let total_tools = metrics.len();

        if total_tools == 0 {
            return QualityHealthStatus::Unknown;
        }

        let system_score = self.system_quality_score();

        if system_score >= 0.9 {
            QualityHealthStatus::Excellent
        } else if system_score >= 0.8 {
            QualityHealthStatus::Good
        } else if system_score >= 0.6 {
            QualityHealthStatus::Degraded
        } else if system_score >= 0.4 {
            QualityHealthStatus::Poor
        } else {
            QualityHealthStatus::Critical
        }
    }

    /// Get tools with quality issues
    pub fn get_problematic_tools(&self) -> Vec<ToolQualityMetrics> {
        self.get_all_metrics()
            .into_iter()
            .filter(|m| m.quality_score < 0.8 || m.error_rate > 0.05)
            .collect()
    }

    /// Get summary report
    pub fn summary_report(&self) -> QualitySummaryReport {
        let metrics = self.get_all_metrics();
        let total_tools = metrics.len();
        let total_invocations: u64 = metrics.iter().map(|m| m.total_invocations).sum();
        let total_errors: u64 = metrics.iter().map(|m| m.failed_invocations).sum();
        let overall_error_rate = if total_invocations > 0 {
            total_errors as f64 / total_invocations as f64
        } else {
            0.0
        };

        let avg_latency: f64 = if !metrics.is_empty() {
            metrics.iter().map(|m| m.avg_latency_ms).sum::<f64>() / metrics.len() as f64
        } else {
            0.0
        };

        QualitySummaryReport {
            uptime_secs: self.start_time.elapsed().as_secs(),
            total_tools,
            total_invocations,
            total_errors,
            overall_error_rate,
            avg_latency_ms: avg_latency,
            system_quality_score: self.system_quality_score(),
            health_status: self.health_status(),
            problematic_tools_count: self.get_problematic_tools().len(),
        }
    }
}

impl Default for QualityRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Health status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityHealthStatus {
    /// Excellent quality (>90%)
    Excellent,
    /// Good quality (80-90%)
    Good,
    /// Degraded quality (60-80%)
    Degraded,
    /// Poor quality (40-60%)
    Poor,
    /// Critical quality (<40%)
    Critical,
    /// Unknown (no data)
    Unknown,
}

impl std::fmt::Display for QualityHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Excellent => write!(f, "excellent"),
            Self::Good => write!(f, "good"),
            Self::Degraded => write!(f, "degraded"),
            Self::Poor => write!(f, "poor"),
            Self::Critical => write!(f, "critical"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Summary report for quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySummaryReport {
    pub uptime_secs: u64,
    pub total_tools: usize,
    pub total_invocations: u64,
    pub total_errors: u64,
    pub overall_error_rate: f64,
    pub avg_latency_ms: f64,
    pub system_quality_score: f64,
    pub health_status: QualityHealthStatus,
    pub problematic_tools_count: usize,
}

/// RAII guard for timing tool invocations
pub struct QualityTimer<'a> {
    registry: &'a QualityRegistry,
    tool_name: &'a str,
    start: Instant,
    success: bool,
}

impl<'a> QualityTimer<'a> {
    /// Create a new quality timer
    pub fn new(registry: &'a QualityRegistry, tool_name: &'a str) -> Self {
        Self {
            registry,
            tool_name,
            start: Instant::now(),
            success: true,
        }
    }

    /// Mark the operation as failed
    pub fn mark_failed(&mut self) {
        self.success = false;
    }
}

impl Drop for QualityTimer<'_> {
    fn drop(&mut self) {
        let latency = self.start.elapsed();
        if self.success {
            self.registry.record_success(self.tool_name, latency);
        } else {
            self.registry.record_failure(self.tool_name, latency);
        }
    }
}

/// Global quality registry
static QUALITY_REGISTRY: std::sync::OnceLock<QualityRegistry> = std::sync::OnceLock::new();

/// Get the global quality registry
pub fn global_quality() -> &'static QualityRegistry {
    QUALITY_REGISTRY.get_or_init(QualityRegistry::with_defaults)
}

/// Macro to time a tool invocation with quality tracking
#[macro_export]
macro_rules! quality_time {
    ($registry:expr, $tool:expr, $body:expr) => {{
        let mut timer = $crate::quality::QualityTimer::new($registry, $tool);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body));
        match result {
            Ok(r) => r,
            Err(_) => {
                timer.mark_failed();
                panic!("Tool invocation panicked");
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_config_default() {
        let config = QualityConfig::default();
        assert_eq!(config.target_error_rate, 0.01);
        assert_eq!(config.target_p95_latency_ms, 1000);
    }

    #[test]
    fn quality_registry_new() {
        let registry = QualityRegistry::with_defaults();
        let metrics = registry.get_all_metrics();
        assert!(metrics.is_empty());
    }

    #[test]
    fn quality_registry_record_success() {
        let registry = QualityRegistry::with_defaults();

        registry.record_success("test_tool", Duration::from_millis(10));
        registry.record_success("test_tool", Duration::from_millis(20));

        let metrics = registry.get_metrics("test_tool").unwrap();
        assert_eq!(metrics.total_invocations, 2);
        assert_eq!(metrics.successful_invocations, 2);
        assert_eq!(metrics.failed_invocations, 0);
    }

    #[test]
    fn quality_registry_record_failure() {
        let registry = QualityRegistry::with_defaults();

        registry.record_success("test_tool", Duration::from_millis(10));
        registry.record_failure("test_tool", Duration::from_millis(5));

        let metrics = registry.get_metrics("test_tool").unwrap();
        assert_eq!(metrics.total_invocations, 2);
        assert_eq!(metrics.failed_invocations, 1);
        assert!(metrics.error_rate > 0.0);
    }

    #[test]
    fn quality_metrics_scores() {
        let registry = QualityRegistry::with_defaults();

        // All successes
        for _ in 0..10 {
            registry.record_success("good_tool", Duration::from_millis(50));
        }

        let metrics = registry.get_metrics("good_tool").unwrap();
        assert!(metrics.quality_score > 0.9);
        assert!(metrics.reliability_score > 0.9);
    }

    #[test]
    fn quality_health_status() {
        let registry = QualityRegistry::with_defaults();

        // No data
        assert_eq!(registry.health_status(), QualityHealthStatus::Unknown);

        // All successes
        for _ in 0..10 {
            registry.record_success("tool1", Duration::from_millis(10));
        }

        assert!(matches!(
            registry.health_status(),
            QualityHealthStatus::Excellent | QualityHealthStatus::Good
        ));
    }

    #[test]
    fn quality_timer_success() {
        let registry = QualityRegistry::with_defaults();

        {
            let _timer = QualityTimer::new(&registry, "timed_tool");
            std::thread::sleep(Duration::from_millis(5));
        }

        let metrics = registry.get_metrics("timed_tool").unwrap();
        assert_eq!(metrics.total_invocations, 1);
        assert_eq!(metrics.successful_invocations, 1);
    }

    #[test]
    fn quality_timer_failure() {
        let registry = QualityRegistry::with_defaults();

        {
            let mut timer = QualityTimer::new(&registry, "failing_tool");
            timer.mark_failed();
        }

        let metrics = registry.get_metrics("failing_tool").unwrap();
        assert_eq!(metrics.total_invocations, 1);
        assert_eq!(metrics.failed_invocations, 1);
    }

    #[test]
    fn summary_report() {
        let registry = QualityRegistry::with_defaults();

        registry.record_success("tool1", Duration::from_millis(10));
        registry.record_success("tool2", Duration::from_millis(20));

        let report = registry.summary_report();
        assert_eq!(report.total_tools, 2);
        assert_eq!(report.total_invocations, 2);
    }

    #[test]
    fn problematic_tools() {
        let registry = QualityRegistry::with_defaults();

        // Good tool
        for _ in 0..10 {
            registry.record_success("good_tool", Duration::from_millis(10));
        }

        // Problematic tool
        for _ in 0..5 {
            registry.record_failure("bad_tool", Duration::from_millis(10));
        }

        let problematic = registry.get_problematic_tools();
        assert_eq!(problematic.len(), 1);
        assert_eq!(problematic[0].tool_name, "bad_tool");
    }

    #[test]
    fn health_status_display() {
        assert_eq!(QualityHealthStatus::Excellent.to_string(), "excellent");
        assert_eq!(QualityHealthStatus::Good.to_string(), "good");
        assert_eq!(QualityHealthStatus::Degraded.to_string(), "degraded");
        assert_eq!(QualityHealthStatus::Poor.to_string(), "poor");
        assert_eq!(QualityHealthStatus::Critical.to_string(), "critical");
        assert_eq!(QualityHealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn global_quality_registry() {
        let registry = global_quality();
        registry.record_success("global_test", Duration::from_millis(1));

        let metrics = registry.get_metrics("global_test");
        assert!(metrics.is_some());
    }
}
