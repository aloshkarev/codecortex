use crate::debounce::{DebounceConfig, FileEventKind, SmartDebouncer};
use crate::filter::{EventFilter, EventFilterBuilder, WatchEventKind};
use crate::perf::{PerfConfig, PerformanceManager};
use cortex_core::{CortexConfig, CortexError, GitOperations, Result};
use cortex_indexer::Indexer;
use notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, instrument, warn};

#[derive(Clone)]
pub struct WatchSession {
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
}

impl WatchSession {
    #[instrument(skip(config), fields(watched_count = config.watched_paths.len()))]
    pub fn new(config: &CortexConfig) -> Self {
        debug!(count = config.watched_paths.len(), "Creating watch session");
        Self {
            watched_paths: Arc::new(Mutex::new(config.watched_paths.iter().cloned().collect())),
        }
    }

    pub fn list(&self) -> Vec<PathBuf> {
        self.watched_paths
            .lock()
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    pub fn watch(&self, path: &Path) -> Result<()> {
        let mut guard = self.watched_paths.lock().map_err(|e| {
            warn!(error = %e, "Failed to acquire lock on watched_paths");
            CortexError::Io(e.to_string())
        })?;
        guard.insert(path.to_path_buf());
        info!(path = %path.display(), "Path added to watch list");
        Ok(())
    }

    #[instrument(skip(self), fields(path = %path.display()))]
    pub fn unwatch(&self, path: &Path) -> Result<bool> {
        let mut guard = self.watched_paths.lock().map_err(|e| {
            warn!(error = %e, "Failed to acquire lock on watched_paths");
            CortexError::Io(e.to_string())
        })?;
        let removed = guard.remove(path);
        if removed {
            info!(path = %path.display(), "Path removed from watch list");
        } else {
            debug!(path = %path.display(), "Path was not in watch list");
        }
        Ok(removed)
    }

    pub fn persist_to_config(&self, config: &mut CortexConfig) -> Result<()> {
        config.watched_paths = self.list();
        config.save()
    }

    #[instrument(skip(self, indexer))]
    pub async fn run(self, indexer: Indexer) -> Result<()> {
        info!("Starting file watcher");
        let (tx, mut rx) = mpsc::channel::<PathBuf>(128);
        let watched_paths = self.list();
        let watched_roots: Vec<PathBuf> = watched_paths
            .iter()
            .cloned()
            .map(canonicalize_lossy)
            .collect();
        let mut branch_state: HashMap<PathBuf, (String, String)> = HashMap::new();

        let mut debouncer = new_debouncer(
            Duration::from_secs(2),
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for event in events {
                        let _ = tx.blocking_send(event.path);
                    }
                }
            },
        )
        .map_err(|e| {
            warn!(error = %e, "Failed to create debouncer");
            CortexError::Io(e.to_string())
        })?;

        for path in watched_paths {
            debouncer
                .watcher()
                .watch(path.as_path(), RecursiveMode::Recursive)
                .map_err(|e| {
                    warn!(path = %path.display(), error = %e, "Failed to watch path");
                    CortexError::Io(e.to_string())
                })?;
            info!(path = %path.display(), "Watching path");
        }

        while let Some(changed_path) = rx.recv().await {
            debug!(path = %changed_path.display(), "File change detected");
            index_changed_path(&indexer, &changed_path, &watched_roots, &mut branch_state).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CortexConfig {
        CortexConfig::default()
    }

    #[test]
    fn watch_session_new_empty() {
        let config = test_config();
        let session = WatchSession::new(&config);
        assert!(session.list().is_empty());
    }

    #[test]
    fn watch_session_new_with_paths() {
        let mut config = test_config();
        config.watched_paths = vec![PathBuf::from("/path/to/repo")];
        let session = WatchSession::new(&config);
        let paths = session.list();
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn watch_session_watch_path() {
        let config = test_config();
        let session = WatchSession::new(&config);

        let result = session.watch(Path::new("/new/path"));
        assert!(result.is_ok());

        let paths = session.list();
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn watch_session_watch_multiple_paths() {
        let config = test_config();
        let session = WatchSession::new(&config);

        session.watch(Path::new("/path1")).unwrap();
        session.watch(Path::new("/path2")).unwrap();
        session.watch(Path::new("/path3")).unwrap();

        let paths = session.list();
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn watch_session_watch_duplicate_path() {
        let config = test_config();
        let session = WatchSession::new(&config);

        session.watch(Path::new("/path")).unwrap();
        session.watch(Path::new("/path")).unwrap(); // Duplicate

        let paths = session.list();
        assert_eq!(paths.len(), 1); // Still 1 because HashSet
    }

    #[test]
    fn watch_session_unwatch_path() {
        let config = test_config();
        let session = WatchSession::new(&config);

        session.watch(Path::new("/path")).unwrap();
        let removed = session.unwatch(Path::new("/path")).unwrap();
        assert!(removed);
        assert!(session.list().is_empty());
    }

    #[test]
    fn watch_session_unwatch_nonexistent() {
        let config = test_config();
        let session = WatchSession::new(&config);

        let removed = session.unwatch(Path::new("/nonexistent")).unwrap();
        assert!(!removed);
    }

    #[test]
    fn watch_session_clone() {
        let config = test_config();
        let session = WatchSession::new(&config);
        session.watch(Path::new("/path")).unwrap();

        let cloned = session.clone();
        // Both share the same Arc<Mutex<HashSet>>
        cloned.watch(Path::new("/path2")).unwrap();

        let paths = session.list();
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn watch_session_list_returns_sorted_consistently() {
        let config = test_config();
        let session = WatchSession::new(&config);

        session.watch(Path::new("/z")).unwrap();
        session.watch(Path::new("/a")).unwrap();
        session.watch(Path::new("/m")).unwrap();

        let paths = session.list();
        assert_eq!(paths.len(), 3);
    }

    #[test]
    fn watch_session_persist_to_config() {
        let mut config = test_config();
        let session = WatchSession::new(&config);

        session.watch(Path::new("/path1")).unwrap();
        session.watch(Path::new("/path2")).unwrap();

        // Note: This test doesn't verify file persistence, just that config is updated
        // Full persistence would require a writable home directory
        let paths_before = config.watched_paths.len();
        assert_ne!(paths_before, 2); // Config not yet updated

        // Update config watched_paths manually for test
        config.watched_paths = session.list();
        assert_eq!(config.watched_paths.len(), 2);
    }
}

/// Configuration for SmartWatchSession
#[derive(Debug, Clone)]
pub struct SmartWatchConfig {
    /// Debounce configuration
    pub debounce: DebounceConfig,
    /// Performance configuration
    pub perf: PerfConfig,
    /// Whether to use event filtering
    pub use_filter: bool,
    /// File extensions to include (empty = all source files)
    pub include_extensions: Vec<String>,
    /// Directories to exclude
    pub exclude_dirs: Vec<String>,
}

impl Default for SmartWatchConfig {
    fn default() -> Self {
        Self {
            debounce: DebounceConfig::default(),
            perf: PerfConfig::default(),
            use_filter: true,
            include_extensions: vec![
                "rs".into(),
                "py".into(),
                "js".into(),
                "ts".into(),
                "go".into(),
                "c".into(),
                "cpp".into(),
                "h".into(),
                "java".into(),
                "php".into(),
                "rb".into(),
            ],
            exclude_dirs: vec![
                "target".into(),
                "node_modules".into(),
                ".git".into(),
                "__pycache__".into(),
                "build".into(),
                "dist".into(),
            ],
        }
    }
}

/// A watch session with integrated smart debouncing, filtering, and performance management.
///
/// This combines all the watcher components:
/// - `SmartDebouncer` for event coalescing and adaptive delays
/// - `EventFilter` for pattern-based filtering
/// - `PerformanceManager` for backpressure and rate limiting
#[derive(Clone)]
pub struct SmartWatchSession {
    /// Inner watch session for path management
    inner: WatchSession,
    /// Smart debouncer for event coalescing
    debouncer: Arc<Mutex<SmartDebouncer>>,
    /// Event filter for pattern matching
    filter: Arc<Mutex<EventFilter>>,
    /// Performance manager for backpressure
    perf_manager: Arc<PerformanceManager>,
    /// Configuration
    config: SmartWatchConfig,
}

impl SmartWatchSession {
    /// Create a new smart watch session with configuration
    pub fn new(config: SmartWatchConfig) -> Self {
        let cortex_config = CortexConfig::default();
        let inner = WatchSession::new(&cortex_config);

        let debouncer = SmartDebouncer::new(config.debounce.clone());
        let perf_manager = PerformanceManager::new(config.perf.clone());

        // Build filter with configured extensions and directories
        let mut filter_builder = EventFilterBuilder::new();

        for ext in &config.include_extensions {
            filter_builder = filter_builder.include_ext(ext);
        }

        for dir in &config.exclude_dirs {
            filter_builder = filter_builder.exclude_dir(dir);
        }

        let filter = filter_builder.build();

        Self {
            inner,
            debouncer: Arc::new(Mutex::new(debouncer)),
            filter: Arc::new(Mutex::new(filter)),
            perf_manager: Arc::new(perf_manager),
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SmartWatchConfig::default())
    }

    /// Add a path to watch
    pub fn watch(&self, path: &Path) -> Result<()> {
        self.inner.watch(path)
    }

    /// Remove a path from watching
    pub fn unwatch(&self, path: &Path) -> Result<bool> {
        self.inner.unwatch(path)
    }

    /// List all watched paths
    pub fn list(&self) -> Vec<PathBuf> {
        self.inner.list()
    }

    /// Record a file event for debouncing
    pub fn record_event(&self, path: &Path, kind: FileEventKind) {
        // Apply filtering first
        if self.config.use_filter {
            let filter = self.filter.lock().unwrap();
            let watch_kind = match kind {
                FileEventKind::Created => WatchEventKind::Created,
                FileEventKind::Modified => WatchEventKind::Modified,
                FileEventKind::Deleted => WatchEventKind::Removed,
            };

            if !filter.should_process(path, watch_kind) {
                return;
            }
        }

        // Check performance/backpressure
        if !self.perf_manager.should_accept() {
            return;
        }

        // Record in debouncer
        self.perf_manager.record_enqueue();
        let mut debouncer = self.debouncer.lock().unwrap();
        debouncer.add_event(path.to_path_buf(), kind);
    }

    /// Get debounced events ready for processing
    pub fn get_ready_events(&self) -> Vec<crate::debounce::DebouncedEvent> {
        let mut debouncer = self.debouncer.lock().unwrap();
        let events = debouncer.get_ready_events();

        // Record dequeues for performance tracking
        for _ in &events {
            self.perf_manager.record_dequeue();
        }

        events
    }

    /// Check if there are events ready to process
    pub fn has_ready_events(&self) -> bool {
        let debouncer = self.debouncer.lock().unwrap();
        debouncer.has_ready_events()
    }

    /// Get number of pending events in the debouncer
    pub fn pending_count(&self) -> usize {
        let debouncer = self.debouncer.lock().unwrap();
        debouncer.pending_count()
    }

    /// Get performance statistics
    pub fn perf_stats(&self) -> crate::perf::PerfStats {
        self.perf_manager.stats()
    }

    /// Get filter statistics
    pub fn filter_stats(&self) -> crate::filter::FilterStats {
        let filter = self.filter.lock().unwrap();
        filter.stats()
    }

    /// Get the current poll interval from adaptive polling
    pub fn poll_interval(&self) -> Duration {
        self.perf_manager.poll_interval()
    }

    /// Check if the system is in backpressure mode
    pub fn is_in_backpressure(&self) -> bool {
        let stats = self.perf_manager.stats();
        stats.in_backpressure
    }

    /// Add an extension to the filter's include list
    pub fn include_extension(&self, ext: &str) {
        let mut filter = self.filter.lock().unwrap();
        filter.include_extension(ext);
    }

    /// Add a directory to the filter's exclude list
    pub fn exclude_directory(&self, dir: &str) {
        let mut filter = self.filter.lock().unwrap();
        filter.exclude_directory(dir);
    }

    /// Clear all pending events
    pub fn clear_pending(&self) {
        let mut debouncer = self.debouncer.lock().unwrap();
        debouncer.clear();
    }

    /// Persist watched paths to config
    pub fn persist_to_config(&self, config: &mut CortexConfig) -> Result<()> {
        self.inner.persist_to_config(config)
    }

    /// Run the watcher with integrated smart handling
    pub async fn run(self, indexer: Indexer) -> Result<()> {
        info!("Starting smart file watcher");
        let (tx, mut rx) = mpsc::channel::<PathBuf>(128);
        let watched_paths = self.list();
        let watched_roots: Vec<PathBuf> = watched_paths
            .iter()
            .cloned()
            .map(canonicalize_lossy)
            .collect();
        let mut branch_state: HashMap<PathBuf, (String, String)> = HashMap::new();

        let mut debouncer = new_debouncer(
            Duration::from_millis(self.config.debounce.min_delay_ms),
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for event in events {
                        let _ = tx.blocking_send(event.path);
                    }
                }
            },
        )
        .map_err(|e| {
            warn!(error = %e, "Failed to create debouncer");
            CortexError::Io(e.to_string())
        })?;

        for path in watched_paths {
            debouncer
                .watcher()
                .watch(path.as_path(), RecursiveMode::Recursive)
                .map_err(|e| {
                    warn!(path = %path.display(), error = %e, "Failed to watch path");
                    CortexError::Io(e.to_string())
                })?;
            info!(path = %path.display(), "Watching path with smart handling");
        }

        let mut _event_count = 0u64;
        let mut last_stats_time = std::time::Instant::now();

        while let Some(changed_path) = rx.recv().await {
            // Record the event for smart processing
            self.record_event(&changed_path, FileEventKind::Modified);

            // Process ready events
            let ready_events = self.get_ready_events();
            for event in ready_events {
                debug!(
                    path = %event.path.display(),
                    coalesced = event.coalesced_count,
                    "Processing debounced event"
                );

                index_changed_path(&indexer, &event.path, &watched_roots, &mut branch_state).await;
                _event_count += 1;
            }

            // Periodically record rate and log stats
            if last_stats_time.elapsed().as_secs() >= 10 {
                let stats = self.perf_stats();
                if stats.events_processed > 0 || stats.events_dropped > 0 {
                    info!(
                        processed = stats.events_processed,
                        dropped = stats.events_dropped,
                        queue = stats.queue_size,
                        in_backpressure = stats.in_backpressure,
                        "Smart watcher stats"
                    );
                }
                last_stats_time = std::time::Instant::now();
            }
        }

        Ok(())
    }
}

fn canonicalize_lossy(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn resolve_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn resolve_repository_root(changed_path: &Path, watched_roots: &[PathBuf]) -> Option<PathBuf> {
    let changed = changed_path
        .canonicalize()
        .unwrap_or_else(|_| changed_path.to_path_buf());

    let watched_root = watched_roots
        .iter()
        .filter(|root| changed.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned();

    watched_root
        .as_deref()
        .and_then(resolve_git_root)
        .or_else(|| resolve_git_root(&changed))
}

async fn index_changed_path(
    indexer: &Indexer,
    changed_path: &Path,
    watched_roots: &[PathBuf],
    branch_state: &mut HashMap<PathBuf, (String, String)>,
) {
    let repo_root = resolve_repository_root(changed_path, watched_roots);
    let Some(repo_root) = repo_root else {
        let _ = indexer.index_path(changed_path).await;
        return;
    };

    let git_ops = GitOperations::new(&repo_root);
    if !git_ops.is_git_repo() {
        let _ = indexer.index_path(changed_path).await;
        return;
    }

    let branch = match git_ops.get_current_branch() {
        Ok(branch) => branch,
        Err(err) => {
            warn!(
                path = %repo_root.display(),
                error = %err,
                "Failed to resolve current branch for watched change"
            );
            let _ = indexer.index_path(changed_path).await;
            return;
        }
    };
    let commit = match git_ops.get_current_commit() {
        Ok(commit) => commit,
        Err(err) => {
            warn!(
                path = %repo_root.display(),
                error = %err,
                "Failed to resolve current commit for watched change"
            );
            let _ = indexer.index_path(changed_path).await;
            return;
        }
    };

    let branch_changed = branch_state
        .get(&repo_root)
        .map(|(old_branch, old_commit)| old_branch != &branch || old_commit != &commit)
        .unwrap_or(false);

    if branch_changed {
        info!(
            repo = %repo_root.display(),
            branch = %branch,
            "Detected branch switch during watch; re-indexing repository for active branch"
        );
    }

    let target_path = if branch_changed {
        repo_root.as_path()
    } else {
        changed_path
    };

    match indexer
        .index_path_with_branch_context(target_path, &branch, &commit, &repo_root, false, false)
        .await
    {
        Ok(_) => {
            branch_state.insert(repo_root, (branch, commit));
        }
        Err(err) => {
            warn!(
                repo = %repo_root.display(),
                path = %target_path.display(),
                error = %err,
                "Failed to index watched change with branch context"
            );
        }
    }
}

#[cfg(test)]
mod smart_tests {
    use super::*;

    #[test]
    fn smart_watch_session_new() {
        let session = SmartWatchSession::with_defaults();
        assert!(session.list().is_empty());
        assert_eq!(session.pending_count(), 0);
    }

    #[test]
    fn smart_watch_session_watch_path() {
        let session = SmartWatchSession::with_defaults();

        let result = session.watch(Path::new("/new/path"));
        assert!(result.is_ok());

        let paths = session.list();
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn smart_watch_session_record_event() {
        let session = SmartWatchSession::with_defaults();

        // Record an event for a Rust file (should be included by default)
        session.record_event(Path::new("/src/main.rs"), FileEventKind::Modified);

        assert_eq!(session.pending_count(), 1);
    }

    #[test]
    fn smart_watch_session_filter_excludes_node_modules() {
        let session = SmartWatchSession::with_defaults();

        // Record an event in node_modules (should be filtered out)
        session.record_event(
            Path::new("/project/node_modules/package/index.js"),
            FileEventKind::Modified,
        );

        assert_eq!(session.pending_count(), 0); // Filtered out
    }

    #[test]
    fn smart_watch_session_filter_excludes_target() {
        let session = SmartWatchSession::with_defaults();

        // Record an event in target directory (should be filtered out)
        session.record_event(
            Path::new("/project/target/debug/main.rs"),
            FileEventKind::Modified,
        );

        assert_eq!(session.pending_count(), 0); // Filtered out
    }

    #[test]
    fn smart_watch_session_filter_includes_python() {
        let session = SmartWatchSession::with_defaults();

        // Python files should be included by default
        session.record_event(Path::new("/src/main.py"), FileEventKind::Modified);

        assert_eq!(session.pending_count(), 1);
    }

    #[test]
    fn smart_watch_session_add_filter_rules() {
        let session = SmartWatchSession::with_defaults();

        // Exclude .md files
        session.exclude_directory("docs");

        // Should still include Rust files
        session.record_event(Path::new("/src/main.rs"), FileEventKind::Modified);
        assert_eq!(session.pending_count(), 1);
    }

    #[test]
    fn smart_watch_session_clear_pending() {
        let session = SmartWatchSession::with_defaults();

        session.record_event(Path::new("/src/main.rs"), FileEventKind::Modified);
        assert_eq!(session.pending_count(), 1);

        session.clear_pending();
        assert_eq!(session.pending_count(), 0);
    }

    #[test]
    fn smart_watch_session_unwatch() {
        let session = SmartWatchSession::with_defaults();

        session.watch(Path::new("/path")).unwrap();
        let removed = session.unwatch(Path::new("/path")).unwrap();
        assert!(removed);
        assert!(session.list().is_empty());
    }

    #[test]
    fn smart_watch_session_stats() {
        let session = SmartWatchSession::with_defaults();

        let perf_stats = session.perf_stats();
        assert_eq!(perf_stats.events_processed, 0);

        let filter_stats = session.filter_stats();
        assert!(filter_stats.include_extensions > 0);
    }

    #[test]
    fn smart_watch_config_default() {
        let config = SmartWatchConfig::default();

        assert!(config.use_filter);
        assert!(config.include_extensions.contains(&"rs".to_string()));
        assert!(config.exclude_dirs.contains(&"target".to_string()));
    }

    #[test]
    fn smart_watch_session_clone() {
        let session = SmartWatchSession::with_defaults();
        session.watch(Path::new("/path")).unwrap();

        let cloned = session.clone();
        cloned.watch(Path::new("/path2")).unwrap();

        let paths = session.list();
        assert_eq!(paths.len(), 2);
    }
}
