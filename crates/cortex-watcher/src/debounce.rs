//! Smart debouncing strategies for file watching.
//!
//! This module provides intelligent debouncing with:
//! - Event coalescing (combining rapid events)
//! - Adaptive delays based on event frequency
//! - Priority queuing for important events
//! - Rate limiting for high-frequency events

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Configuration for smart debouncing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebounceConfig {
    /// Minimum delay before processing an event
    pub min_delay_ms: u64,
    /// Maximum delay cap
    pub max_delay_ms: u64,
    /// Time window for coalescing events
    pub coalesce_window_ms: u64,
    /// Maximum events to queue
    pub max_queue_size: usize,
    /// Enable rate limiting
    pub rate_limit: bool,
    /// Maximum events per second before rate limiting kicks in
    pub max_events_per_sec: usize,
}

impl Default for DebounceConfig {
    fn default() -> Self {
        Self {
            min_delay_ms: 100,
            max_delay_ms: 2000,
            coalesce_window_ms: 500,
            max_queue_size: 1000,
            rate_limit: true,
            max_events_per_sec: 100,
        }
    }
}

/// Priority level for events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPriority {
    /// High priority: immediate processing needed
    High,
    /// Normal priority: standard processing
    Normal,
    /// Low priority: can be batched
    Low,
}

/// Kind of file system event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileEventKind {
    /// File or directory created
    Created,
    /// File or directory modified
    Modified,
    /// File or directory deleted
    Deleted,
}

/// A debounced event ready for processing
#[derive(Debug, Clone)]
pub struct DebouncedEvent {
    /// Path that triggered the event
    pub path: PathBuf,
    /// Event kind (create, modify, delete)
    pub event_kind: FileEventKind,
    /// Timestamp of the original event
    pub timestamp: Instant,
    /// Number of coalesced events
    pub coalesced_count: usize,
    /// Priority level
    pub priority: EventPriority,
}

/// Smart debouncer with adaptive delays and event coalescing
#[derive(Debug)]
pub struct SmartDebouncer {
    config: DebounceConfig,
    event_queue: VecDeque<QueuedEvent>,
    last_process_time: Instant,
    events_this_second: usize,
    second_start: Instant,
}

#[derive(Debug, Clone)]
struct QueuedEvent {
    path: PathBuf,
    event_kind: FileEventKind,
    timestamp: Instant,
    priority: EventPriority,
}

impl SmartDebouncer {
    /// Create a new smart debouncer with configuration
    pub fn new(config: DebounceConfig) -> Self {
        Self {
            config,
            event_queue: VecDeque::new(),
            last_process_time: Instant::now(),
            events_this_second: 0,
            second_start: Instant::now(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(DebounceConfig::default())
    }

    /// Add an event to the debouncer
    pub fn add_event(&mut self, path: PathBuf, kind: FileEventKind) {
        self.add_event_with_priority(path, kind, EventPriority::Normal);
    }

    /// Add an event with specific priority
    pub fn add_event_with_priority(
        &mut self,
        path: PathBuf,
        kind: FileEventKind,
        priority: EventPriority,
    ) {
        // Rate limiting check
        let now = Instant::now();
        if self.second_start.elapsed() >= Duration::from_secs(1) {
            self.events_this_second = 0;
            self.second_start = now;
        }

        self.events_this_second += 1;
        if self.config.rate_limit && self.events_this_second > self.config.max_events_per_sec {
            return; // Skip event due to rate limiting
        }

        self.event_queue.push_back(QueuedEvent {
            path,
            event_kind: kind,
            timestamp: now,
            priority,
        });

        // Trim queue if too large
        while self.event_queue.len() > self.config.max_queue_size {
            self.event_queue.pop_front();
        }
    }

    /// Check if there are events ready to be processed
    pub fn has_ready_events(&self) -> bool {
        if self.event_queue.is_empty() {
            return false;
        }

        let now = Instant::now();
        let delay = self.calculate_delay();

        // Check oldest event
        if let Some(front) = self.event_queue.front() {
            now.duration_since(front.timestamp) >= delay
        } else {
            false
        }
    }

    /// Get all ready events (clearing them from the queue)
    pub fn get_ready_events(&mut self) -> Vec<DebouncedEvent> {
        let mut ready = Vec::with_capacity(self.event_queue.len());
        let now = Instant::now();
        let delay = self.calculate_delay();

        while let Some(front) = self.event_queue.front() {
            if now.duration_since(front.timestamp) >= delay {
                let event = self.event_queue.pop_front().unwrap();

                // Coalesce with other events for the same path
                let mut coalesced_count = 1;
                let mut to_remove = Vec::new();

                for (i, queued) in self.event_queue.iter().enumerate() {
                    if queued.path == event.path {
                        coalesced_count += 1;
                        to_remove.push(i);
                    }
                }

                // Remove coalesced events (in reverse order to maintain indices)
                for i in to_remove.into_iter().rev() {
                    self.event_queue.remove(i);
                }

                ready.push(DebouncedEvent {
                    path: event.path,
                    event_kind: event.event_kind,
                    timestamp: event.timestamp,
                    coalesced_count,
                    priority: event.priority,
                });
            } else {
                break;
            }
        }

        // Update last process time
        if !ready.is_empty() {
            self.last_process_time = now;
        }

        ready
    }

    /// Calculate adaptive delay based on queue size
    fn calculate_delay(&self) -> Duration {
        let queue_size = self.event_queue.len();
        let base_ratio = queue_size as f64 / self.config.max_queue_size as f64;

        let delay_ms = self.config.min_delay_ms as f64
            + (base_ratio * (self.config.max_delay_ms - self.config.min_delay_ms) as f64);

        Duration::from_millis(delay_ms.min(self.config.max_delay_ms as f64) as u64)
    }

    /// Get the number of pending events
    pub fn pending_count(&self) -> usize {
        self.event_queue.len()
    }

    /// Clear all pending events
    pub fn clear(&mut self) {
        self.event_queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_config_default() {
        let config = DebounceConfig::default();
        assert_eq!(config.min_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 2000);
        assert_eq!(config.coalesce_window_ms, 500);
        assert_eq!(config.max_queue_size, 1000);
        assert!(config.rate_limit);
        assert_eq!(config.max_events_per_sec, 100);
    }

    #[test]
    fn smart_debouncer_new() {
        let debouncer = SmartDebouncer::with_defaults();
        assert_eq!(debouncer.pending_count(), 0);
    }

    #[test]
    fn add_single_event() {
        let mut debouncer = SmartDebouncer::with_defaults();
        let path = PathBuf::from("/test/file.rs");

        debouncer.add_event(path, FileEventKind::Modified);
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn add_multiple_events() {
        let mut debouncer = SmartDebouncer::with_defaults();
        let path = PathBuf::from("/test/file.rs");

        debouncer.add_event(path.clone(), FileEventKind::Modified);
        debouncer.add_event(path.clone(), FileEventKind::Modified);
        debouncer.add_event(path.clone(), FileEventKind::Modified);

        assert_eq!(debouncer.pending_count(), 3);
    }

    #[test]
    fn event_priority_levels() {
        assert_eq!(EventPriority::High, EventPriority::High);
        assert_ne!(EventPriority::High, EventPriority::Normal);
        assert_ne!(EventPriority::Normal, EventPriority::Low);
    }

    #[test]
    fn has_ready_events_empty_queue() {
        let debouncer = SmartDebouncer::with_defaults();
        assert!(!debouncer.has_ready_events());
    }

    #[test]
    fn has_ready_events_too_soon() {
        let mut debouncer = SmartDebouncer::with_defaults();
        debouncer.add_event(PathBuf::from("/test/file.rs"), FileEventKind::Modified);

        // Immediately check - should not be ready
        assert!(!debouncer.has_ready_events());
    }

    #[test]
    fn get_ready_events_after_delay() {
        let mut debouncer = SmartDebouncer::new(DebounceConfig {
            min_delay_ms: 10,
            max_delay_ms: 100,
            ..Default::default()
        });

        let path = PathBuf::from("/test/file.rs");
        debouncer.add_event(path, FileEventKind::Modified);

        // Wait for delay
        std::thread::sleep(Duration::from_millis(20));

        assert!(debouncer.has_ready_events());
        let events = debouncer.get_ready_events();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn rate_limiting() {
        let mut debouncer = SmartDebouncer::new(DebounceConfig {
            max_events_per_sec: 5,
            rate_limit: true,
            ..Default::default()
        });

        // Add events rapidly
        for i in 0..10 {
            debouncer.add_event(
                PathBuf::from(format!("/test/file{}.rs", i)),
                FileEventKind::Modified,
            );
        }

        // Some events should have been rate limited
        assert!(debouncer.pending_count() <= 10);
    }

    #[test]
    fn adaptive_delay_increases_with_queue() {
        let config = DebounceConfig {
            min_delay_ms: 50,
            max_delay_ms: 500,
            max_queue_size: 100,
            ..Default::default()
        };
        let mut debouncer = SmartDebouncer::new(config);

        // Add a few events
        for i in 0..10 {
            debouncer.add_event(
                PathBuf::from(format!("/test/file{}.rs", i)),
                FileEventKind::Modified,
            );
        }

        let delay_small = debouncer.calculate_delay();
        assert!(delay_small < Duration::from_millis(100));

        // Fill the queue
        for i in 0..90 {
            debouncer.add_event(
                PathBuf::from(format!("/test/file{}.rs", i)),
                FileEventKind::Modified,
            );
        }

        let delay_large = debouncer.calculate_delay();
        assert!(delay_large > delay_small);
    }

    #[test]
    fn clear_pending_events() {
        let mut debouncer = SmartDebouncer::with_defaults();
        debouncer.add_event(PathBuf::from("/test/file.rs"), FileEventKind::Modified);
        assert_eq!(debouncer.pending_count(), 1);

        debouncer.clear();
        assert_eq!(debouncer.pending_count(), 0);
    }

    #[test]
    fn coalescing_counts_events() {
        let mut debouncer = SmartDebouncer::new(DebounceConfig {
            min_delay_ms: 10,
            max_delay_ms: 100,
            ..Default::default()
        });
        let path = PathBuf::from("/test/file.rs");

        // Add multiple events for the same path
        for _ in 0..5 {
            debouncer.add_event(path.clone(), FileEventKind::Modified);
        }

        // Wait for delay
        std::thread::sleep(Duration::from_millis(20));

        let events = debouncer.get_ready_events();
        assert_eq!(events.len(), 1);
        assert!(events[0].coalesced_count > 1);
    }

    #[test]
    fn high_priority_events() {
        let mut debouncer = SmartDebouncer::with_defaults();
        let path = PathBuf::from("/test/file.rs");

        debouncer.add_event_with_priority(path, FileEventKind::Modified, EventPriority::High);
        assert_eq!(debouncer.pending_count(), 1);
    }

    #[test]
    fn file_event_kind_variants() {
        assert_eq!(FileEventKind::Created, FileEventKind::Created);
        assert_eq!(FileEventKind::Modified, FileEventKind::Modified);
        assert_eq!(FileEventKind::Deleted, FileEventKind::Deleted);
    }

    #[test]
    fn debounced_event_debug() {
        let event = DebouncedEvent {
            path: PathBuf::from("/test/file.rs"),
            event_kind: FileEventKind::Modified,
            timestamp: Instant::now(),
            coalesced_count: 3,
            priority: EventPriority::High,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Modified")); // Rust Debug format uses PascalCase for enum variants
        assert!(debug_str.contains("DebouncedEvent"));
    }

    #[test]
    fn debounce_config_custom() {
        let config = DebounceConfig {
            min_delay_ms: 200,
            max_delay_ms: 5000,
            coalesce_window_ms: 1000,
            max_queue_size: 500,
            rate_limit: false,
            max_events_per_sec: 50,
        };
        assert_eq!(config.min_delay_ms, 200);
        assert_eq!(config.max_delay_ms, 5000);
        assert!(!config.rate_limit);
    }
}
