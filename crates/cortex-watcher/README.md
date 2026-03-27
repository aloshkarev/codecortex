# cortex-watcher

> `cortex-watcher` provides file system watching and project registry management for CodeCortex. It monitors repository trees for changes, debounces and filters file events, and maintains a persistent SQLite-backed registry of multi-project state including git branch tracking.

## What it does

- Watches repository directories recursively via `notify` OS events
- Debounces and coalesces file events with adaptive delays to avoid redundant re-indexing
- Filters events by extension, size, and path (including `.gitignore`-style patterns)
- Manages a persistent project registry (SQLite via `rusqlite`) with git branch state and status tracking
- Detects remote/mounted filesystems and adjusts polling behavior accordingly
- Provides rate limiting and backpressure to handle high-churn repositories

## Features

| Feature | Description |
|---------|-------------|
| `WatchSession` | Basic path watching session |
| `SmartWatchSession` | Recommended: integrates smart debouncing, filtering, and performance management |
| `SmartDebouncer` | Adaptive debounce with configurable min/max delay and coalesce window |
| `EventFilter` | Include/exclude filters by extension, directory, glob, and file size |
| `ProjectRegistry` | Persistent multi-project registry backed by SQLite (`~/.cortex/projects.db`) |
| `PerformanceManager` | Rate limiting, backpressure, and adaptive polling under high-churn conditions |
| Remote FS detection | Identifies NFS, SSHFS, and other remote mounts via `is_remote_path()` |

## Project registry

The `ProjectRegistry` is the authoritative source of truth for multi-repository workflows. It is used by `cortex-cli` scope resolution to determine:

- The current active project for project-scoped analysis
- Whether to fall back to all-project scope when no active project is set
- Git branch and status information for each registered project

Registry data is persisted to `~/.cortex/projects.db` (SQLite).

```rust
use cortex_watcher::ProjectRegistry;
use std::path::Path;

let mut registry = ProjectRegistry::new()?;

// Register a project
let project = registry.add_project("/path/to/repo")?;
println!("Added: {} (branch: {})", project.path.display(), project.branch);

// List all projects
for p in registry.list_projects() {
    println!("  {} — {}", p.path.display(), p.status);
}

// Set the active project
registry.set_current_project(Some("/path/to/repo"))?;

// Refresh git state
registry.refresh_project("/path/to/repo")?;
```

## Smart watch session (recommended)

```rust
use cortex_watcher::{SmartWatchSession, SmartWatchConfig};
use std::path::Path;

let session = SmartWatchSession::with_defaults();
session.watch(Path::new("/path/to/repo"))?;

// Record events (auto-filtered and debounced)
session.record_event(Path::new("/src/main.rs"), FileEventKind::Modified);

// Poll for ready events after debounce delay
let ready = session.get_ready_events();
for event in ready {
    println!("Processing: {} (coalesced {}x)", event.path.display(), event.coalesced_count);
}

// Performance stats
let stats = session.perf_stats();
println!("Processed: {}, Dropped: {}", stats.events_processed, stats.events_dropped);
```

## Smart debouncing

```rust
use cortex_watcher::{SmartDebouncer, DebounceConfig, EventPriority};

let config = DebounceConfig {
    min_delay_ms: 100,
    max_delay_ms: 1000,
    coalesce_window_ms: 50,
    ..Default::default()
};
let debouncer = SmartDebouncer::new(config);
debouncer.add_event(PathBuf::from("/src/lib.rs"), FileEventKind::Modified);
let ready = debouncer.get_ready_events();
```

## Event filtering

```rust
use cortex_watcher::{EventFilterBuilder, WatchEventKind};
use std::path::Path;

let filter = EventFilterBuilder::new()
    .include_ext("rs")
    .include_ext("py")
    .exclude_dir("target")
    .exclude_dir("node_modules")
    .max_size(10 * 1024 * 1024)
    .build();

if filter.should_process(Path::new("src/main.rs"), WatchEventKind::Modified) {
    // process the event
}
```

## Performance tuning

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

if manager.should_accept() {
    manager.record_enqueue();
    // process
    manager.record_dequeue();
}
```

## Remote filesystem detection

```rust
use cortex_watcher::is_remote_path;

if is_remote_path("/Volumes/remote") {
    // Use polling-based watcher, not inotify/kqueue
}
```

## Dependencies

- `notify` — OS-level filesystem event notifications
- `notify-debouncer-mini` — Debounce adapter for `notify`
- `rusqlite` — SQLite persistence for project registry
- `dashmap` — Concurrent map for thread-safe event tracking
- `parking_lot` — High-performance synchronization primitives
- `cortex-core` — Config and shared models
- `cortex-indexer` — Called on ready events to trigger re-indexing

## Tests

```bash
cargo test -p cortex-watcher -- --test-threads=1
```

Current test count: **85 tests**
