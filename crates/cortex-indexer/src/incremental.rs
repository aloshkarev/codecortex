//! Incremental Indexing Support
//!
//! Provides file change detection for efficient re-indexing:
//! - Content hash-based change detection
//! - Persistent hash cache with revision tracking
//! - Git-aware incremental indexing
//! - File modification time optimization

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Hash cache entry for tracking file changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashEntry {
    /// SHA-256 hash of file content
    pub content_hash: String,
    /// File size in bytes
    pub file_size: u64,
    /// Last modification time (Unix timestamp)
    pub modified_time: u64,
    /// When this entry was cached
    pub cached_at: u64,
    /// Repository revision when indexed
    pub repo_revision: String,
}

/// Incremental indexing manager
#[derive(Debug)]
pub struct IncrementalIndexer {
    /// Path to the hash cache database
    cache_path: PathBuf,
    /// In-memory cache of file hashes
    hash_cache: HashMap<String, HashEntry>,
    /// Whether to use mtime optimization
    use_mtime_optimization: bool,
    /// Current repository revision
    current_revision: String,
}

impl IncrementalIndexer {
    /// Create a new incremental indexer with default cache location
    pub fn new() -> Self {
        Self::with_cache_path(Self::default_cache_path())
    }

    /// Create an incremental indexer with a custom cache path
    pub fn with_cache_path(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            hash_cache: HashMap::new(),
            use_mtime_optimization: true,
            current_revision: String::new(),
        }
    }

    /// Get the default cache path
    pub fn default_cache_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".cortex/incremental_cache.json")
    }

    /// Set the current repository revision
    pub fn set_revision(&mut self, revision: impl Into<String>) {
        self.current_revision = revision.into();
    }

    /// Enable or disable mtime optimization
    pub fn set_mtime_optimization(&mut self, enabled: bool) {
        self.use_mtime_optimization = enabled;
    }

    /// Calculate the hash of file content
    pub fn hash_content(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check if a file has changed since last indexing
    pub fn has_file_changed(&self, path: &Path, content: &str) -> bool {
        let path_key = path.to_string_lossy().to_string();

        // Check if we have a cached entry
        let Some(entry) = self.hash_cache.get(&path_key) else {
            return true; // No cache entry, file is "changed"
        };

        // Check revision - if repo changed, force reindex
        if entry.repo_revision != self.current_revision && !self.current_revision.is_empty() {
            return true;
        }

        // Quick check: file size
        let current_size = content.len() as u64;
        if current_size != entry.file_size {
            return true;
        }

        // Content hash comparison
        let current_hash = Self::hash_content(content);
        current_hash != entry.content_hash
    }

    /// Check if a file has changed using mtime optimization
    pub fn has_file_changed_fast(&self, path: &Path, content: &str) -> bool {
        if !self.use_mtime_optimization {
            return self.has_file_changed(path, content);
        }

        let path_key = path.to_string_lossy().to_string();

        // Check if we have a cached entry
        let Some(entry) = self.hash_cache.get(&path_key) else {
            return true;
        };

        // Check revision
        if entry.repo_revision != self.current_revision && !self.current_revision.is_empty() {
            return true;
        }

        // Check file size first (fast)
        let current_size = content.len() as u64;
        if current_size != entry.file_size {
            return true;
        }

        // Check mtime if available (fast path)
        if let Ok(metadata) = std::fs::metadata(path)
            && let Ok(modified) = metadata.modified()
            && let Ok(modified_ts) = modified.duration_since(SystemTime::UNIX_EPOCH)
            && modified_ts.as_secs() == entry.modified_time
        {
            // If mtime hasn't changed, file hasn't changed
            return false;
        }

        // Fall back to content hash
        let current_hash = Self::hash_content(content);
        current_hash != entry.content_hash
    }

    /// Record a file as indexed
    pub fn record_file(&mut self, path: &Path, content: &str) {
        let path_key = path.to_string_lossy().to_string();
        let content_hash = Self::hash_content(content);

        let modified_time = std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.hash_cache.insert(
            path_key,
            HashEntry {
                content_hash,
                file_size: content.len() as u64,
                modified_time,
                cached_at: now,
                repo_revision: self.current_revision.clone(),
            },
        );
    }

    /// Remove a file from the cache
    pub fn remove_file(&mut self, path: &Path) {
        let path_key = path.to_string_lossy().to_string();
        self.hash_cache.remove(&path_key);
    }

    /// Filter files to only those that have changed
    pub fn filter_changed_files<'a>(
        &self,
        files: &'a [PathBuf],
        read_content: impl Fn(&Path) -> Option<String>,
    ) -> Vec<(&'a PathBuf, String)> {
        files
            .iter()
            .filter_map(|path| {
                let content = read_content(path)?;
                if self.has_file_changed_fast(path, &content) {
                    Some((path, content))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get the number of cached files
    pub fn cache_size(&self) -> usize {
        self.hash_cache.len()
    }

    /// Clear the entire cache
    pub fn clear_cache(&mut self) {
        self.hash_cache.clear();
    }

    /// Save the cache to disk
    pub fn save(&self) -> std::io::Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&self.hash_cache)
            .map_err(std::io::Error::other)?;
        std::fs::write(&self.cache_path, json)
    }

    /// Load the cache from disk
    pub fn load(&mut self) -> std::io::Result<()> {
        if !self.cache_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&self.cache_path)?;
        let cache: HashMap<String, HashEntry> = serde_json::from_str(&content)
            .map_err(std::io::Error::other)?;

        self.hash_cache = cache;
        Ok(())
    }

    /// Get statistics about the cache
    pub fn stats(&self) -> IncrementalStats {
        let total_entries = self.hash_cache.len();
        let total_size: u64 = self.hash_cache.values().map(|e| e.file_size).sum();

        IncrementalStats {
            cached_files: total_entries,
            total_cached_bytes: total_size,
            current_revision: self.current_revision.clone(),
        }
    }

    /// Invalidate entries for a specific repository
    pub fn invalidate_repo(&mut self, repo_path: &str) {
        let prefix = format!("{}/", repo_path.replace('\\', "/"));
        self.hash_cache.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Prune entries for files that no longer exist
    pub fn prune_missing_files(&mut self) -> usize {
        let before = self.hash_cache.len();

        self.hash_cache.retain(|path, _| {
            PathBuf::from(path).exists()
        });

        before - self.hash_cache.len()
    }
}

impl Default for IncrementalIndexer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the incremental index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalStats {
    /// Number of cached files
    pub cached_files: usize,
    /// Total bytes cached
    pub total_cached_bytes: u64,
    /// Current repository revision
    pub current_revision: String,
}

/// Change detection result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeStatus {
    /// File is new (not in cache)
    New,
    /// File has been modified
    Modified,
    /// File is unchanged
    Unchanged,
    /// File was deleted
    Deleted,
}

/// Git-aware incremental indexing support
pub struct GitAwareIncremental {
    inner: IncrementalIndexer,
    git_command: String,
}

impl GitAwareIncremental {
    /// Create a new git-aware incremental indexer
    pub fn new(repo_path: &Path) -> Self {
        let mut inner = IncrementalIndexer::new();

        // Try to get current git revision
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            && output.status.success()
        {
            let revision = String::from_utf8_lossy(&output.stdout);
            inner.set_revision(revision.trim().to_string());
        }

        Self {
            inner,
            git_command: "git".to_string(),
        }
    }

    /// Get the inner incremental indexer
    pub fn inner(&self) -> &IncrementalIndexer {
        &self.inner
    }

    /// Get mutable access to the inner indexer
    pub fn inner_mut(&mut self) -> &mut IncrementalIndexer {
        &mut self.inner
    }

    /// Get files changed since a git reference
    pub fn get_changed_since_ref(&self, repo_path: &Path, git_ref: &str) -> Vec<PathBuf> {
        let output = std::process::Command::new(&self.git_command)
            .args(["diff", "--name-only", git_ref])
            .current_dir(repo_path)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter_map(|line| {
                        let path = repo_path.join(line.trim());
                        if path.exists() { Some(path) } else { None }
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Get files changed in the working directory (unstaged + staged)
    pub fn get_uncommitted_changes(&self, repo_path: &Path) -> Vec<PathBuf> {
        let mut changed = Vec::new();

        // Get unstaged changes
        if let Ok(output) = std::process::Command::new(&self.git_command)
            .args(["diff", "--name-only"])
            .current_dir(repo_path)
            .output()
            && output.status.success()
        {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let path = repo_path.join(line.trim());
                if path.exists() {
                    changed.push(path);
                }
            }
        }

        // Get staged changes
        if let Ok(output) = std::process::Command::new(&self.git_command)
            .args(["diff", "--cached", "--name-only"])
            .current_dir(repo_path)
            .output()
            && output.status.success()
        {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let path = repo_path.join(line.trim());
                if path.exists() && !changed.contains(&path) {
                    changed.push(path);
                }
            }
        }

        changed
    }

    /// Update the stored revision to current HEAD
    pub fn update_revision(&mut self, repo_path: &Path) {
        if let Ok(output) = std::process::Command::new(&self.git_command)
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            && output.status.success()
        {
            let revision = String::from_utf8_lossy(&output.stdout);
            self.inner.set_revision(revision.trim().to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn incremental_indexer_new() {
        let indexer = IncrementalIndexer::new();
        assert_eq!(indexer.cache_size(), 0);
    }

    #[test]
    fn hash_content_consistent() {
        let content = "test content";
        let hash1 = IncrementalIndexer::hash_content(content);
        let hash2 = IncrementalIndexer::hash_content(content);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn hash_content_different() {
        let hash1 = IncrementalIndexer::hash_content("content 1");
        let hash2 = IncrementalIndexer::hash_content("content 2");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn record_and_check_file() {
        let mut indexer = IncrementalIndexer::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Create and record file
        std::fs::write(&file_path, "test content").unwrap();
        indexer.record_file(&file_path, "test content");

        // Should not be changed with same content
        assert!(!indexer.has_file_changed(&file_path, "test content"));

        // Should be changed with different content
        assert!(indexer.has_file_changed(&file_path, "different content"));
    }

    #[test]
    fn new_file_is_changed() {
        let indexer = IncrementalIndexer::new();
        let path = Path::new("/nonexistent/path.txt");

        assert!(indexer.has_file_changed(path, "any content"));
    }

    #[test]
    fn remove_file() {
        let mut indexer = IncrementalIndexer::new();
        let path = Path::new("/test/file.txt");

        indexer.record_file(path, "content");
        assert_eq!(indexer.cache_size(), 1);

        indexer.remove_file(path);
        assert_eq!(indexer.cache_size(), 0);
    }

    #[test]
    fn clear_cache() {
        let mut indexer = IncrementalIndexer::new();

        indexer.record_file(Path::new("/file1.txt"), "content1");
        indexer.record_file(Path::new("/file2.txt"), "content2");
        assert_eq!(indexer.cache_size(), 2);

        indexer.clear_cache();
        assert_eq!(indexer.cache_size(), 0);
    }

    #[test]
    fn save_and_load_cache() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("cache.json");

        // Create and populate indexer
        let mut indexer = IncrementalIndexer::with_cache_path(cache_path.clone());
        indexer.set_revision("abc123");
        indexer.record_file(Path::new("/test/file.txt"), "content");
        indexer.save().unwrap();

        // Load into new indexer
        let mut indexer2 = IncrementalIndexer::with_cache_path(cache_path);
        indexer2.load().unwrap();

        assert_eq!(indexer2.cache_size(), 1);
        assert!(!indexer2.has_file_changed(Path::new("/test/file.txt"), "content"));
    }

    #[test]
    fn stats() {
        let mut indexer = IncrementalIndexer::new();
        indexer.set_revision("rev123");
        indexer.record_file(Path::new("/file.txt"), "content");

        let stats = indexer.stats();
        assert_eq!(stats.cached_files, 1);
        assert!(stats.total_cached_bytes > 0);
        assert_eq!(stats.current_revision, "rev123");
    }

    #[test]
    fn revision_invalidation() {
        let mut indexer = IncrementalIndexer::new();
        indexer.set_revision("rev1");
        indexer.record_file(Path::new("/file.txt"), "content");

        // Should be unchanged with same revision
        assert!(!indexer.has_file_changed(Path::new("/file.txt"), "content"));

        // Change revision
        indexer.set_revision("rev2");

        // Now should be considered changed
        assert!(indexer.has_file_changed(Path::new("/file.txt"), "content"));
    }

    #[test]
    fn mtime_optimization() {
        let mut indexer = IncrementalIndexer::new();
        indexer.set_mtime_optimization(true);

        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "content").unwrap();

        indexer.record_file(&file_path, "content");

        // Should use fast path and not detect change
        assert!(!indexer.has_file_changed_fast(&file_path, "content"));
    }

    #[test]
    fn change_status_variants() {
        assert_eq!(ChangeStatus::New, ChangeStatus::New);
        assert_ne!(ChangeStatus::New, ChangeStatus::Modified);
        assert_ne!(ChangeStatus::Modified, ChangeStatus::Unchanged);
        assert_ne!(ChangeStatus::Unchanged, ChangeStatus::Deleted);
    }

    #[test]
    fn incremental_stats_default() {
        let stats = IncrementalStats {
            cached_files: 0,
            total_cached_bytes: 0,
            current_revision: String::new(),
        };
        assert_eq!(stats.cached_files, 0);
    }
}
