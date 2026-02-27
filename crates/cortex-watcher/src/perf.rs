//! Performance Tuning for File Watching
//!
//! Provides performance optimization for high-load scenarios:
//! - Backpressure handling
//! - Memory-bounded event queues
//! - Adaptive polling intervals
//! - Resource monitoring and throttling

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for performance tuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfConfig {
    /// Maximum events to queue before applying backpressure
    pub max_queue_size: usize,
    /// High water mark for queue (start dropping low-priority events)
    pub high_water_mark: f64,
    /// Low water mark for queue (stop dropping)
    pub low_water_mark: f64,
    /// Base polling interval
    pub base_poll_interval_ms: u64,
    /// Minimum polling interval under load
    pub min_poll_interval_ms: u64,
    /// Maximum polling interval when idle
    pub max_poll_interval_ms: u64,
    /// Enable adaptive polling
    pub adaptive_polling: bool,
    /// Memory limit for event queue (bytes)
    pub memory_limit_bytes: usize,
    /// Cooldown period after backpressure event
    pub backpressure_cooldown_ms: u64,
    /// Maximum events per second before throttling
    pub max_events_per_sec: usize,
}

impl Default for PerfConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10_000,
            high_water_mark: 0.8,
            low_water_mark: 0.5,
            base_poll_interval_ms: 100,
            min_poll_interval_ms: 10,
            max_poll_interval_ms: 1000,
            adaptive_polling: true,
            memory_limit_bytes: 50 * 1024 * 1024, // 50 MB
            backpressure_cooldown_ms: 1000,
            max_events_per_sec: 1000,
        }
    }
}

/// Performance statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerfStats {
    /// Current queue size
    pub queue_size: usize,
    /// Events processed total
    pub events_processed: u64,
    /// Events dropped due to backpressure
    pub events_dropped: u64,
    /// Times backpressure was applied
    pub backpressure_events: u64,
    /// Current events per second
    pub current_eps: f64,
    /// Average processing latency (ms)
    pub avg_latency_ms: f64,
    /// Current poll interval (ms)
    pub current_poll_interval_ms: u64,
    /// Memory used by queue (estimated bytes)
    pub memory_used_bytes: usize,
    /// Whether currently in backpressure mode
    pub in_backpressure: bool,
}

/// Backpressure controller
#[derive(Debug)]
pub struct BackpressureController {
    config: PerfConfig,
    /// Current queue size
    queue_size: Arc<AtomicUsize>,
    /// Events dropped counter
    events_dropped: Arc<AtomicU64>,
    /// Backpressure events counter
    backpressure_events: Arc<AtomicU64>,
    /// Whether in backpressure mode
    in_backpressure: Arc<AtomicBool>,
    /// Last backpressure time
    last_backpressure: Arc<std::sync::Mutex<Option<Instant>>>,
    /// Events in current second
    events_this_second: Arc<AtomicUsize>,
    /// Second start time
    second_start: Arc<std::sync::Mutex<Instant>>,
}

impl BackpressureController {
    /// Create a new backpressure controller
    pub fn new(config: PerfConfig) -> Self {
        Self {
            config,
            queue_size: Arc::new(AtomicUsize::new(0)),
            events_dropped: Arc::new(AtomicU64::new(0)),
            backpressure_events: Arc::new(AtomicU64::new(0)),
            in_backpressure: Arc::new(AtomicBool::new(false)),
            last_backpressure: Arc::new(std::sync::Mutex::new(None)),
            events_this_second: Arc::new(AtomicUsize::new(0)),
            second_start: Arc::new(std::sync::Mutex::new(Instant::now())),
        }
    }

    /// Check if we should accept an event
    pub fn should_accept(&self) -> bool {
        // Check rate limit
        let now = Instant::now();
        {
            let mut second_start = self.second_start.lock().unwrap();
            if now.duration_since(*second_start) >= Duration::from_secs(1) {
                self.events_this_second.store(0, Ordering::Relaxed);
                *second_start = now;
            }
        }

        let events = self.events_this_second.fetch_add(1, Ordering::Relaxed);
        if events >= self.config.max_events_per_sec {
            self.events_dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Check queue size
        let queue_size = self.queue_size.load(Ordering::Relaxed);
        let high_water = (self.config.max_queue_size as f64 * self.config.high_water_mark) as usize;

        if queue_size >= high_water {
            // Enter backpressure mode
            if !self.in_backpressure.swap(true, Ordering::Relaxed) {
                self.backpressure_events.fetch_add(1, Ordering::Relaxed);
                *self.last_backpressure.lock().unwrap() = Some(now);
            }
            self.events_dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Check if we should exit backpressure
        if self.in_backpressure.load(Ordering::Relaxed) {
            let low_water = (self.config.max_queue_size as f64 * self.config.low_water_mark) as usize;
            if queue_size <= low_water
                && let Some(last) = *self.last_backpressure.lock().unwrap()
                && now.duration_since(last) >= Duration::from_millis(self.config.backpressure_cooldown_ms)
            {
                self.in_backpressure.store(false, Ordering::Relaxed);
            }
        }

        true
    }

    /// Record an event being added to queue
    pub fn record_enqueue(&self) {
        self.queue_size.fetch_add(1, Ordering::Relaxed);
    }

    /// Record an event being removed from queue
    pub fn record_dequeue(&self) {
        self.queue_size.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current queue size
    pub fn queue_size(&self) -> usize {
        self.queue_size.load(Ordering::Relaxed)
    }

    /// Check if in backpressure mode
    pub fn is_in_backpressure(&self) -> bool {
        self.in_backpressure.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> BackpressureStats {
        BackpressureStats {
            queue_size: self.queue_size.load(Ordering::Relaxed),
            events_dropped: self.events_dropped.load(Ordering::Relaxed),
            backpressure_events: self.backpressure_events.load(Ordering::Relaxed),
            in_backpressure: self.in_backpressure.load(Ordering::Relaxed),
        }
    }
}

/// Statistics from backpressure controller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackpressureStats {
    pub queue_size: usize,
    pub events_dropped: u64,
    pub backpressure_events: u64,
    pub in_backpressure: bool,
}

/// Adaptive poll interval calculator
#[derive(Debug)]
pub struct AdaptivePoller {
    config: PerfConfig,
    current_interval_ms: Arc<AtomicU64>,
    /// Recent event counts for adaptation
    recent_rates: Arc<std::sync::Mutex<Vec<f64>>>,
}

impl AdaptivePoller {
    /// Create a new adaptive poller
    pub fn new(config: PerfConfig) -> Self {
        let base_interval = config.base_poll_interval_ms;
        Self {
            config,
            current_interval_ms: Arc::new(AtomicU64::new(base_interval)),
            recent_rates: Arc::new(std::sync::Mutex::new(Vec::with_capacity(10))),
        }
    }

    /// Get the current poll interval
    pub fn current_interval(&self) -> Duration {
        Duration::from_millis(self.current_interval_ms.load(Ordering::Relaxed))
    }

    /// Record events per second and adapt
    pub fn record_rate(&self, events_per_sec: f64) {
        {
            let mut rates = self.recent_rates.lock().unwrap();
            rates.push(events_per_sec);
            if rates.len() > 10 {
                rates.remove(0);
            }
        }

        self.adapt();
    }

    /// Adapt the poll interval based on recent rates
    fn adapt(&self) {
        if !self.config.adaptive_polling {
            return;
        }

        let rates = self.recent_rates.lock().unwrap();
        if rates.is_empty() {
            return;
        }

        let avg_rate: f64 = rates.iter().sum::<f64>() / rates.len() as f64;

        let new_interval = if avg_rate > 100.0 {
            // High event rate - poll faster
            self.config.min_poll_interval_ms
        } else if avg_rate > 50.0 {
            // Medium rate
            (self.config.base_poll_interval_ms / 2).max(self.config.min_poll_interval_ms)
        } else if avg_rate > 10.0 {
            // Normal rate
            self.config.base_poll_interval_ms
        } else if avg_rate > 1.0 {
            // Low rate - poll slower
            (self.config.base_poll_interval_ms * 2).min(self.config.max_poll_interval_ms)
        } else {
            // Very low rate - poll slowest
            self.config.max_poll_interval_ms
        };

        self.current_interval_ms.store(new_interval, Ordering::Relaxed);
    }

    /// Get current interval in ms
    pub fn current_interval_ms(&self) -> u64 {
        self.current_interval_ms.load(Ordering::Relaxed)
    }
}

/// Memory-bounded event queue
#[derive(Debug)]
pub struct BoundedEventQueue<T> {
    events: Vec<T>,
    max_size: usize,
    memory_limit: usize,
    estimated_size_per_event: usize,
}

impl<T> BoundedEventQueue<T> {
    /// Create a new bounded event queue
    pub fn new(max_size: usize, memory_limit: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_size.min(1000)),
            max_size,
            memory_limit,
            estimated_size_per_event: std::mem::size_of::<T>(),
        }
    }

    /// Try to push an event
    pub fn push(&mut self, event: T) -> Result<(), T> {
        if self.events.len() >= self.max_size {
            return Err(event);
        }

        let estimated_memory = self.events.len() * self.estimated_size_per_event;
        if estimated_memory >= self.memory_limit {
            return Err(event);
        }

        self.events.push(event);
        Ok(())
    }

    /// Pop an event
    pub fn pop(&mut self) -> Option<T> {
        self.events.pop()
    }

    /// Get the front event
    pub fn front(&self) -> Option<&T> {
        self.events.first()
    }

    /// Get current length
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Check if at capacity
    pub fn is_full(&self) -> bool {
        self.events.len() >= self.max_size
    }

    /// Get estimated memory usage
    pub fn estimated_memory(&self) -> usize {
        self.events.len() * self.estimated_size_per_event
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Drain all events
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.events.drain(..)
    }
}

/// Resource monitor for watching system resources
#[allow(dead_code)]
pub struct ResourceMonitor {
    /// Maximum CPU percentage before throttling
    max_cpu_percent: f64,
    /// Maximum memory usage before throttling
    max_memory_percent: f64,
    /// Whether currently throttled
    throttled: Arc<AtomicBool>,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(max_cpu_percent: f64, max_memory_percent: f64) -> Self {
        Self {
            max_cpu_percent,
            max_memory_percent,
            throttled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if throttled
    pub fn is_throttled(&self) -> bool {
        self.throttled.load(Ordering::Relaxed)
    }

    /// Update resource usage (simplified - in production would read actual system stats)
    pub fn update(&self, _cpu_percent: f64, _memory_percent: f64) {
        // In a real implementation, this would check actual system resources
        // For now, we just track the throttled state
    }

    /// Get the throttled flag for sharing
    pub fn throttled_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.throttled)
    }
}

impl Default for ResourceMonitor {
    fn default() -> Self {
        Self::new(80.0, 80.0)
    }
}

/// Performance manager combining all tuning components
pub struct PerformanceManager {
    config: PerfConfig,
    backpressure: BackpressureController,
    poller: AdaptivePoller,
    events_processed: Arc<AtomicU64>,
    start_time: Instant,
}

impl PerformanceManager {
    /// Create a new performance manager
    pub fn new(config: PerfConfig) -> Self {
        let backpressure = BackpressureController::new(config.clone());
        let poller = AdaptivePoller::new(config.clone());

        Self {
            config,
            backpressure,
            poller,
            events_processed: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }

    /// Check if an event should be accepted
    pub fn should_accept(&self) -> bool {
        self.backpressure.should_accept()
    }

    /// Record an event being enqueued
    pub fn record_enqueue(&self) {
        self.backpressure.record_enqueue();
    }

    /// Record an event being dequeued
    pub fn record_dequeue(&self) {
        self.backpressure.record_dequeue();
        self.events_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the current poll interval
    pub fn poll_interval(&self) -> Duration {
        self.poller.current_interval()
    }

    /// Record events per second rate
    pub fn record_rate(&self, eps: f64) {
        self.poller.record_rate(eps);
    }

    /// Get comprehensive statistics
    pub fn stats(&self) -> PerfStats {
        let bp_stats = self.backpressure.stats();
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let processed = self.events_processed.load(Ordering::Relaxed);
        let current_eps = if elapsed > 0.0 {
            processed as f64 / elapsed
        } else {
            0.0
        };

        PerfStats {
            queue_size: bp_stats.queue_size,
            events_processed: processed,
            events_dropped: bp_stats.events_dropped,
            backpressure_events: bp_stats.backpressure_events,
            current_eps,
            avg_latency_ms: 0.0, // Would need timing wrapper
            current_poll_interval_ms: self.poller.current_interval_ms(),
            memory_used_bytes: bp_stats.queue_size * std::mem::size_of::<usize>(),
            in_backpressure: bp_stats.in_backpressure,
        }
    }

    /// Get the configuration
    pub fn config(&self) -> &PerfConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_config_default() {
        let config = PerfConfig::default();
        assert_eq!(config.max_queue_size, 10_000);
        assert!(config.adaptive_polling);
    }

    #[test]
    fn backpressure_controller_new() {
        let config = PerfConfig::default();
        let controller = BackpressureController::new(config);
        assert_eq!(controller.queue_size(), 0);
        assert!(!controller.is_in_backpressure());
    }

    #[test]
    fn backpressure_accept_under_limit() {
        let config = PerfConfig {
            max_queue_size: 100,
            max_events_per_sec: 1000,
            ..Default::default()
        };
        let controller = BackpressureController::new(config);

        assert!(controller.should_accept());
    }

    #[test]
    fn adaptive_poller_new() {
        let config = PerfConfig::default();
        let poller = AdaptivePoller::new(config);
        assert_eq!(poller.current_interval(), Duration::from_millis(100));
    }

    #[test]
    fn adaptive_poller_adapts() {
        let config = PerfConfig {
            base_poll_interval_ms: 100,
            min_poll_interval_ms: 10,
            max_poll_interval_ms: 1000,
            adaptive_polling: true,
            ..Default::default()
        };
        let poller = AdaptivePoller::new(config);

        // Record high rate
        poller.record_rate(150.0);
        assert_eq!(poller.current_interval_ms(), 10);

        // Record low rate
        for _ in 0..15 {
            poller.record_rate(0.5);
        }
        assert_eq!(poller.current_interval_ms(), 1000);
    }

    #[test]
    fn bounded_queue_basic() {
        let mut queue: BoundedEventQueue<i32> = BoundedEventQueue::new(10, 1024);

        assert!(queue.push(1).is_ok());
        assert!(queue.push(2).is_ok());
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn bounded_queue_full() {
        let mut queue: BoundedEventQueue<i32> = BoundedEventQueue::new(2, 1024);

        assert!(queue.push(1).is_ok());
        assert!(queue.push(2).is_ok());
        assert!(queue.push(3).is_err());
        assert!(queue.is_full());
    }

    #[test]
    fn bounded_queue_pop() {
        let mut queue: BoundedEventQueue<i32> = BoundedEventQueue::new(10, 1024);

        queue.push(1).unwrap();
        queue.push(2).unwrap();

        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.pop(), Some(1));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn bounded_queue_clear() {
        let mut queue: BoundedEventQueue<i32> = BoundedEventQueue::new(10, 1024);

        queue.push(1).unwrap();
        queue.push(2).unwrap();
        queue.clear();

        assert!(queue.is_empty());
    }

    #[test]
    fn resource_monitor_new() {
        let monitor = ResourceMonitor::new(80.0, 80.0);
        assert!(!monitor.is_throttled());
    }

    #[test]
    fn performance_manager_new() {
        let config = PerfConfig::default();
        let manager = PerformanceManager::new(config);

        let stats = manager.stats();
        assert_eq!(stats.events_processed, 0);
        assert_eq!(stats.queue_size, 0);
    }

    #[test]
    fn performance_manager_events() {
        let config = PerfConfig::default();
        let manager = PerformanceManager::new(config);

        manager.record_enqueue();
        manager.record_dequeue();
        manager.record_enqueue();
        manager.record_dequeue();

        let stats = manager.stats();
        assert_eq!(stats.events_processed, 2);
    }

    #[test]
    fn perf_stats_default() {
        let stats = PerfStats::default();
        assert_eq!(stats.queue_size, 0);
        assert_eq!(stats.events_processed, 0);
    }
}
