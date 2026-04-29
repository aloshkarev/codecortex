## 2024-05-24 - Avoid `HashMap::entry(key.clone())` on Hot Paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every single iteration because the key is cloned before checking if it exists.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to significantly improve performance by avoiding clones for existing keys.
