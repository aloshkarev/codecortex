## 2026-05-26 - HashMap lookup string cloning optimization
**Learning:** `HashMap::entry(key.clone()).or_insert(...)` unnecessarily clones strings even when the key already exists, causing high memory allocation overhead in hot loops like term frequencies in TF-IDF indexing.
**Action:** Always use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` on performance-critical hot paths instead of `entry()`.
