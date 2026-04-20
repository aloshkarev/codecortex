//! Cache Hierarchy for CodeCortex MCP Tools
//!
//! Implements a two-level cache hierarchy:
//! - L1: In-memory cache using DashMap for fast lookups
//! - L2: Disk-based cache for persistence across restarts
//!
//! Cache invalidation is driven by repository revision changes.

#![allow(dead_code)]

use dashmap::DashMap;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::any::Any;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Default TTL for L1 cache entries (10-60 seconds)
const DEFAULT_L1_TTL: Duration = Duration::from_secs(30);

/// Maximum number of entries in L1 cache
const DEFAULT_L1_MAX_ENTRIES: usize = 10_000;

/// A cached entry with expiration
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    /// The cached value
    pub value: T,
    /// When this entry was created
    pub created_at: Instant,
    /// Time-to-live for this entry
    pub ttl: Duration,
    /// Repository revision this entry is valid for
    pub repo_revision: String,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
    pub fn new(value: T, ttl: Duration, repo_revision: String) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            ttl,
            repo_revision,
        }
    }

    /// Check if this entry has expired
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    /// Check if this entry is valid for the given revision
    pub fn is_valid_for_revision(&self, revision: &str) -> bool {
        self.repo_revision == revision
    }
}

/// L1 Cache: In-memory cache using DashMap for concurrent access
pub struct L1Cache {
    /// The underlying storage
    store: DashMap<String, Box<dyn Any + Send + Sync>>,
    /// Default TTL for entries
    default_ttl: Duration,
    /// Maximum number of entries
    max_entries: usize,
    /// Entry metadata for TTL tracking
    metadata: DashMap<String, (Instant, Duration, String)>, // (created_at, ttl, repo_revision)
}

impl std::fmt::Debug for L1Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("L1Cache")
            .field("store_len", &self.store.len())
            .field("default_ttl", &self.default_ttl)
            .field("max_entries", &self.max_entries)
            .field("metadata_len", &self.metadata.len())
            .finish()
    }
}

impl L1Cache {
    /// Create a new L1 cache with default settings
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
            default_ttl: DEFAULT_L1_TTL,
            max_entries: DEFAULT_L1_MAX_ENTRIES,
            metadata: DashMap::new(),
        }
    }

    /// Create a new L1 cache with custom settings
    pub fn with_settings(default_ttl: Duration, max_entries: usize) -> Self {
        Self {
            store: DashMap::new(),
            default_ttl,
            max_entries,
            metadata: DashMap::new(),
        }
    }

    /// Generate a cache key from components
    pub fn make_key(tool: &str, repo_path: &str, params_hash: &str) -> String {
        format!("{}:{}:{}", tool, repo_path, params_hash)
    }

    /// Get a value from the cache
    pub fn get<T: 'static + Send + Sync + Clone>(&self, key: &str, repo_revision: &str) -> Option<T> {
        // Check metadata by reference to avoid cloning when we will return None
        let meta = self.metadata.get(key)?;
        let (created_at, ttl, cached_revision) = meta.value();
        if created_at.elapsed() > *ttl {
            drop(meta);
            self.remove(key);
            return None;
        }
        if cached_revision != repo_revision {
            drop(meta);
            self.remove(key);
            return None;
        }
        drop(meta);

        // Get the value
        let any_val = self.store.get(key)?;
        any_val.downcast_ref::<T>().cloned()
    }

    /// Put a value into the cache
    pub fn put<T: 'static + Send + Sync + Clone>(&self, key: &str, value: T, repo_revision: &str) {
        // Evict old entries if at capacity
        if self.store.len() >= self.max_entries {
            self.evict_expired();
            if self.store.len() >= self.max_entries {
                // Still at capacity, remove oldest entries
                self.evict_oldest(self.max_entries / 10);
            }
        }

        let key_owned = key.to_string();
        let rev_owned = repo_revision.to_string();
        self.store.insert(key_owned.clone(), Box::new(value));
        self.metadata
            .insert(key_owned, (Instant::now(), self.default_ttl, rev_owned));
    }

    /// Put a value with a custom TTL
    pub fn put_with_ttl<T: 'static + Send + Sync + Clone>(
        &self,
        key: &str,
        value: T,
        ttl: Duration,
        repo_revision: &str,
    ) {
        // Evict old entries if at capacity
        if self.store.len() >= self.max_entries {
            self.evict_expired();
        }

        let key_owned = key.to_string();
        let rev_owned = repo_revision.to_string();
        self.store.insert(key_owned.clone(), Box::new(value));
        self.metadata
            .insert(key_owned, (Instant::now(), ttl, rev_owned));
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &str) {
        self.store.remove(key);
        self.metadata.remove(key);
    }

    /// Clear all entries
    pub fn clear(&self) {
        self.store.clear();
        self.metadata.clear();
    }

    /// Invalidate all entries for a specific repository
    pub fn invalidate_repo(&self, repo_path: &str) {
        let prefix = format!("{}:", repo_path);
        self.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Evict expired entries
    fn evict_expired(&self) {
        let now = Instant::now();
        let expired: Vec<String> = self
            .metadata
            .iter()
            .filter(|m| {
                let (created_at, ttl, _) = m.value();
                now.duration_since(*created_at) > *ttl
            })
            .map(|m| m.key().clone())
            .collect();

        for key in expired {
            self.remove(&key);
        }
    }

    /// Evict the oldest entries
    fn evict_oldest(&self, count: usize) {
        let mut entries: Vec<(String, Instant)> = self
            .metadata
            .iter()
            .map(|m| {
                let (created_at, _, _) = m.value();
                (m.key().clone(), *created_at)
            })
            .collect();

        entries.sort_by_key(|(_, t)| *t);

        for (key, _) in entries.into_iter().take(count) {
            self.remove(&key);
        }
    }

    /// Retain entries matching a predicate
    fn retain<F>(&self, mut predicate: F)
    where
        F: FnMut(&str, &Box<dyn Any + Send + Sync>) -> bool,
    {
        let to_remove: Vec<String> = self
            .store
            .iter()
            .filter(|e| !predicate(e.key(), e.value()))
            .map(|e| e.key().clone())
            .collect();

        for key in to_remove {
            self.remove(&key);
        }
    }
}

impl Default for L1Cache {
    fn default() -> Self {
        Self::new()
    }
}

/// L2 Cache: Disk-based cache for persistence
#[derive(Debug, Clone)]
pub struct L2Cache {
    /// Base path for cache files
    base_path: PathBuf,
}

impl L2Cache {
    /// Create a new L2 cache at the given path
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Create a new L2 cache at the default location
    pub fn default_path() -> Self {
        let base_path = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            base_path: base_path.join(".cortex/cache"),
        }
    }

    /// Get the cache file path for a key
    fn get_path(&self, key: &str) -> PathBuf {
        // Hash the key to create a safe filename
        let hash = format!("{:x}", md5::compute(key));
        self.base_path.join(&hash)
    }

    /// Ensure the cache directory exists
    fn ensure_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.base_path)
    }

    /// Get a value from the cache
    pub fn get<T: DeserializeOwned>(&self, key: &str, repo_revision: &str) -> Option<T> {
        let path = self.get_path(key);
        if !path.exists() {
            return None;
        }

        let content = std::fs::read(&path).ok()?;
        let entry: L2CacheEntry<T> = serde_json::from_slice(&content).ok()?;

        // Check revision
        if entry.repo_revision != repo_revision {
            // Entry is stale, remove it
            let _ = std::fs::remove_file(path);
            return None;
        }

        // Check TTL
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        if now > entry.expires_at {
            let _ = std::fs::remove_file(path);
            return None;
        }

        Some(entry.value)
    }

    /// Put a value into the cache
    pub fn put<T: Serialize>(&self, key: &str, value: T, repo_revision: &str, ttl: Duration) {
        if self.ensure_dir().is_err() {
            return;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let entry = L2CacheEntry {
            value,
            repo_revision: repo_revision.to_string(),
            created_at: now,
            expires_at: now + ttl.as_secs(),
        };

        if let Ok(content) = serde_json::to_vec(&entry) {
            let path = self.get_path(key);
            let _ = std::fs::write(path, content);
        }
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &str) {
        let path = self.get_path(key);
        let _ = std::fs::remove_file(path);
    }

    /// Clear all entries
    pub fn clear(&self) {
        if self.base_path.exists() {
            let _ = std::fs::remove_dir_all(&self.base_path);
            let _ = self.ensure_dir();
        }
    }

    /// Invalidate all entries for a specific repository
    pub fn invalidate_repo(&self, _repo_path: &str) {
        // For L2, we'd need to scan all files and check their content
        // This is expensive, so for now we just note it
        // In practice, revision-based invalidation handles this
    }
}

/// Entry stored in L2 cache
#[derive(Debug, Serialize, Deserialize)]
struct L2CacheEntry<T> {
    value: T,
    repo_revision: String,
    created_at: u64,
    expires_at: u64,
}

/// Cache hierarchy combining L1 and L2 caches
#[derive(Debug)]
pub struct CacheHierarchy {
    /// L1 in-memory cache
    l1: Arc<L1Cache>,
    /// L2 disk-based cache
    l2: L2Cache,
}

impl CacheHierarchy {
    /// Create a new cache hierarchy with default settings
    pub fn new() -> Self {
        Self {
            l1: Arc::new(L1Cache::new()),
            l2: L2Cache::default_path(),
        }
    }

    /// Create a new cache hierarchy with custom settings
    pub fn with_settings(l1_ttl: Duration, l1_max_entries: usize, l2_path: PathBuf) -> Self {
        Self {
            l1: Arc::new(L1Cache::with_settings(l1_ttl, l1_max_entries)),
            l2: L2Cache::new(l2_path),
        }
    }

    /// Get a value from the cache hierarchy
    /// Returns (value, hit_level) where hit_level is "l1", "l2", or "none"
    pub fn get<T: 'static + DeserializeOwned + Serialize + Clone + Send + Sync>(
        &self,
        key: &str,
        repo_revision: &str,
    ) -> (Option<T>, &'static str) {
        // Try L1 first
        if let Some(value) = self.l1.get(key, repo_revision) {
            return (Some(value), "l1");
        }

        // Try L2
        if let Some(value) = self.l2.get::<T>(key, repo_revision) {
            // Populate L1 for faster subsequent access
            self.l1.put(key, value.clone(), repo_revision);
            return (Some(value), "l2");
        }

        (None, "none")
    }

    /// Put a value into the cache hierarchy
    pub fn put<T: 'static + Serialize + Clone + Send + Sync>(&self, key: &str, value: T, repo_revision: &str) {
        // Put in both L1 and L2
        self.l1.put(key, value.clone(), repo_revision);
        self.l2
            .put(key, value, repo_revision, Duration::from_secs(300)); // 5 min L2 TTL
    }

    /// Put a value with custom TTLs
    pub fn put_with_ttls<T: 'static + Serialize + Clone + Send + Sync>(
        &self,
        key: &str,
        value: T,
        repo_revision: &str,
        l1_ttl: Duration,
        l2_ttl: Duration,
    ) {
        self.l1.put_with_ttl(key, value.clone(), l1_ttl, repo_revision);
        self.l2.put(key, value, repo_revision, l2_ttl);
    }

    /// Remove a value from all cache levels
    pub fn remove(&self, key: &str) {
        self.l1.remove(key);
        self.l2.remove(key);
    }

    /// Clear all cache levels
    pub fn clear(&self) {
        self.l1.clear();
        self.l2.clear();
    }

    /// Invalidate all entries for a repository
    pub fn invalidate_repo(&self, repo_path: &str) {
        self.l1.invalidate_repo(repo_path);
        self.l2.invalidate_repo(repo_path);
    }

    /// Get the L1 cache for direct access
    pub fn l1(&self) -> &L1Cache {
        &self.l1
    }

    /// Get the L2 cache for direct access
    pub fn l2(&self) -> &L2Cache {
        &self.l2
    }

    /// Get statistics about the cache
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            l1_entries: self.l1.len(),
            l2_path: self.l2.base_path.clone(),
        }
    }
}

impl Default for CacheHierarchy {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CacheHierarchy {
    fn clone(&self) -> Self {
        Self {
            l1: Arc::clone(&self.l1),
            l2: self.l2.clone(),
        }
    }
}

/// Statistics about the cache
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in L1 cache
    pub l1_entries: usize,
    /// Path to L2 cache directory
    pub l2_path: PathBuf,
}

/// Simple MD5 implementation for key hashing
mod md5 {
    pub fn compute(input: &str) -> u128 {
        // Simple hash for cache keys - not cryptographically secure
        let mut hash: u128 = 0;
        for (i, byte) in input.bytes().enumerate() {
            let shift = (i % 16) * 8;
            hash ^= (byte as u128) << shift;
        }
        // Add some mixing using wrapping multiplication
        hash.wrapping_mul(0x5851F42D4C957F2D)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l1_cache_basic_operations() {
        let cache = L1Cache::new();

        cache.put("key1", "value1".to_string(), "rev1");

        let value: Option<String> = cache.get("key1", "rev1");
        assert_eq!(value, Some("value1".to_string()));
    }

    #[test]
    fn l1_cache_returns_none_for_expired() {
        let cache = L1Cache::with_settings(Duration::from_millis(1), 100);

        cache.put("key1", "value1".to_string(), "rev1");

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(5));

        let value: Option<String> = cache.get("key1", "rev1");
        assert!(value.is_none());
    }

    #[test]
    fn l1_cache_returns_none_for_wrong_revision() {
        let cache = L1Cache::new();

        cache.put("key1", "value1".to_string(), "rev1");

        let value: Option<String> = cache.get("key1", "rev2");
        assert!(value.is_none());
    }

    #[test]
    fn l1_cache_remove() {
        let cache = L1Cache::new();

        cache.put("key1", "value1".to_string(), "rev1");
        cache.remove("key1");

        let value: Option<String> = cache.get("key1", "rev1");
        assert!(value.is_none());
    }

    #[test]
    fn l1_cache_clear() {
        let cache = L1Cache::new();

        cache.put("key1", "value1".to_string(), "rev1");
        cache.put("key2", "value2".to_string(), "rev1");
        cache.clear();

        assert!(cache.is_empty());
    }

    #[test]
    fn l1_cache_make_key() {
        let key = L1Cache::make_key("get_context_capsule", "/repo/path", "abc123");
        assert_eq!(key, "get_context_capsule:/repo/path:abc123");
    }

    #[test]
    fn cache_hierarchy_get_miss() {
        let cache = CacheHierarchy::new();

        let (value, hit): (Option<String>, _) = cache.get("missing_key", "rev1");
        assert!(value.is_none());
        assert_eq!(hit, "none");
    }

    #[test]
    fn cache_hierarchy_put_and_get() {
        let cache = CacheHierarchy::new();

        cache.put("key1", "value1".to_string(), "rev1");

        let (value, hit): (Option<String>, _) = cache.get("key1", "rev1");
        assert_eq!(value, Some("value1".to_string()));
        assert_eq!(hit, "l1"); // First hit should be from L1
    }

    #[test]
    fn cache_hierarchy_remove() {
        let cache = CacheHierarchy::new();

        cache.put("key1", "value1".to_string(), "rev1");
        cache.remove("key1");

        let (value, _): (Option<String>, &'static str) = cache.get("key1", "rev1");
        assert!(value.is_none());
    }

    #[test]
    fn cache_stats() {
        let cache = CacheHierarchy::new();

        cache.put("key1", "value1".to_string(), "rev1");
        cache.put("key2", "value2".to_string(), "rev1");

        let stats = cache.stats();
        assert_eq!(stats.l1_entries, 2);
    }
}
