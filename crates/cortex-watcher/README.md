# cortex-watcher

File system watching and project registry management.

## Overview

This crate provides file watching functionality for monitoring code changes and managing multiple projects.

## Features

- **Recursive Directory Watching**: Monitor all files in a directory tree
- **Smart Debouncing**: Event coalescing with adaptive delays and rate limiting
- **Project Registry**: Manage multiple repositories with persistence
- **Git Integration**: Automatic branch detection and state tracking
- **Remote FS Detection**: Identify and handle remote/mounted filesystems
- **Event Filtering**: Configurable filters with glob patterns
- **Performance Tuning**: Backpressure handling and adaptive polling

## Usage

### Watch Session

```rust
use cortex_watcher::WatchSession;
use cortex_core::CortexConfig;
use std::path::Path;

let config = CortexConfig::default();
let session = WatchSession::new(&config);

// Add paths to watch
session.watch(Path::new("/path/to/repo"))?;

// Get watched paths
let paths = session.list();
println!("Watching {} paths", paths.len());
```

### Project Registry

```rust
use cortex_watcher::ProjectRegistry;
use std::path::Path;

let mut registry = ProjectRegistry::new()?;

// Add a project
let project = registry.add_project("/path/to/repo")?;
println!("Added project: {:?}", project);

// List all projects
let projects = registry.list_projects();
for p in &projects {
    println!("  - {} ({})", p.path.display(), p.status);
}

// Set current project
registry.set_current_project(Some("/path/to/repo"))?;

// Refresh git info
registry.refresh_project("/path/to/repo")?;
```

### Smart Debouncing

```rust
use cortex_watcher::{SmartDebouncer, DebounceConfig, EventPriority};

let config = DebounceConfig {
    delay_ms: 100,
    max_delay_ms: 1000,
    coalesce_window_ms: 50,
    ..Default::default()
};
let debouncer = SmartDebouncer::with_config(config);

// Record events
debouncer.record_event("/path/to/file.rs", EventPriority::Normal);

// Get ready events after delay
let ready = debouncer.get_ready_events();
```

### Event Filtering

```rust
use cortex_watcher::{EventFilter, EventFilterBuilder, WatchEventKind};
use std::path::Path;

let filter = EventFilterBuilder::new()
    .include_ext("rs")
    .include_ext("py")
    .exclude_dir("target")
    .exclude_dir("node_modules")
    .max_size(10 * 1024 * 1024)  // 10MB
    .build();

// Check if event should be processed
if filter.should_process(Path::new("src/main.rs"), WatchEventKind::Modified) {
    // Process the event
}
```

### Performance Tuning

```rust
use cortex_watcher::{PerformanceManager, PerfConfig};

let config = PerfConfig {
    max_queue_size: 5000,
    high_water_mark: 0.8,
    low_water_mark: 0.5,
    adaptive_polling: true,
    max_events_per_sec: 1000,
    ..Default::default()
};
let manager = PerformanceManager::new(config);

// Check if event should be accepted (rate limiting + backpressure)
if manager.should_accept() {
    manager.record_enqueue();
    // Process event
    manager.record_dequeue();
}

// Get stats
let stats = manager.stats();
println!("Queue: {}, Dropped: {}", stats.queue_size, stats.events_dropped);
```

## Remote Filesystem Detection

```rust
use cortex_watcher::{is_remote_path, RemoteFsType};

if is_remote_path("/Volumes/remote") {
    println!("Remote filesystem detected");
}
```

## Dependencies

- `notify` - File system notifications
- `cortex-core` - Core types

## Tests

Run tests with:
```bash
cargo test -p cortex-watcher -- --test-threads=1
```

Current test count: **60 tests**
