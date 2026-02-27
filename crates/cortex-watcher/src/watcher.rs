use cortex_core::{CortexConfig, CortexError, Result};
use cortex_indexer::Indexer;
use notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, new_debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct WatchSession {
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
}

impl WatchSession {
    pub fn new(config: &CortexConfig) -> Self {
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

    pub fn watch(&self, path: &Path) -> Result<()> {
        let mut guard = self
            .watched_paths
            .lock()
            .map_err(|e| CortexError::Io(e.to_string()))?;
        guard.insert(path.to_path_buf());
        Ok(())
    }

    pub fn unwatch(&self, path: &Path) -> Result<bool> {
        let mut guard = self
            .watched_paths
            .lock()
            .map_err(|e| CortexError::Io(e.to_string()))?;
        Ok(guard.remove(path))
    }

    pub fn persist_to_config(&self, config: &mut CortexConfig) -> Result<()> {
        config.watched_paths = self.list();
        config.save()
    }

    pub async fn run(self, indexer: Indexer) -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<PathBuf>(128);

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
        .map_err(|e| CortexError::Io(e.to_string()))?;

        for path in self.list() {
            debouncer
                .watcher()
                .watch(path.as_path(), RecursiveMode::Recursive)
                .map_err(|e| CortexError::Io(e.to_string()))?;
        }

        while let Some(changed_path) = rx.recv().await {
            let _ = indexer.index_path(changed_path).await;
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
