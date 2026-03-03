//! # CodeCortex Watcher Library
//!
//! File system watching and project registry management.
//!
//! ## Overview
//!
//! This crate provides file watching functionality:
//!
//! - **Watch Session**: [`WatchSession`] for monitoring file changes
//! - **Project Registry**: [`ProjectRegistry`] for managing multiple projects
//! - **Git Integration**: Automatic branch detection and state tracking
//! - **Smart Debouncing**: [`SmartDebouncer`] with event coalescing and rate limiting
//! - **Remote FS Support**: [`is_remote_path`] for detecting remote/mounted filesystems
//! - **Event Filtering**: [`EventFilter`] with configurable glob patterns
//! - **Performance Tuning**: [`PerformanceManager`] with backpressure and adaptive polling
//!
//! ## Features
//!
//! - Recursive directory watching
//! - Configurable file filters with glob patterns
//! - Git-aware project state tracking
//! - Persistent project registry
//! - Smart debouncing with adaptive delays
//! - Remote filesystem detection
//! - Memory-bounded event queues
//! - Backpressure handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use cortex_watcher::{WatchSession, SmartDebouncer, DebounceConfig};
//! use cortex_core::CortexConfig;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = CortexConfig::default();
//! let session = WatchSession::new(&config);
//!
//! // Add a path to watch
//! session.watch(Path::new("/path/to/repo"))?;
//!
//! // Get list of watched paths
//! let paths = session.list();
//!
//! // Create a smart debouncer for event handling
//! let debouncer = SmartDebouncer::with_defaults();
//! # Ok(())
//! # }
//! ```
//!
//! ## Event Filtering
//!
//! ```rust
//! use cortex_watcher::{EventFilter, EventFilterBuilder, WatchEventKind};
//! use std::path::Path;
//!
//! let filter = EventFilterBuilder::new()
//!     .include_ext("rs")
//!     .exclude_dir("target")
//!     .build();
//!
//! // Check if event should be processed
//! if filter.should_process(Path::new("src/main.rs"), WatchEventKind::Modified) {
//!     // Process the event
//! }
//! ```
//!
//! ## Performance Tuning
//!
//! ```rust
//! use cortex_watcher::{PerformanceManager, PerfConfig};
//!
//! let config = PerfConfig {
//!     max_queue_size: 5000,
//!     adaptive_polling: true,
//!     ..Default::default()
//! };
//! let manager = PerformanceManager::new(config);
//!
//! // Check if event should be accepted
//! if manager.should_accept() {
//!     manager.record_enqueue();
//!     // Process event
//! }
//! ```

mod debounce;
pub mod filter;
pub mod perf;
mod registry;
pub mod remote;
mod watcher;

pub use debounce::{DebounceConfig, DebouncedEvent, EventPriority, FileEventKind, SmartDebouncer};
pub use filter::{
    EventFilter, EventFilterBuilder, FilterConfig, FilterRule, FilterStats, WatchEventKind,
};
pub use perf::{
    AdaptivePoller, BackpressureController, BackpressureStats, BoundedEventQueue, PerfConfig,
    PerfStats, PerformanceManager, ResourceMonitor,
};
pub use registry::{ProjectRegistry, RegistryError};
pub use remote::{RemoteFsConfig, RemoteFsType, is_remote_path};
pub use watcher::{SmartWatchConfig, SmartWatchSession, WatchSession};
