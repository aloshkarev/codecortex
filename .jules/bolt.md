
## 2024-05-17 - Optimize HashMap insertions in hot paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration because it forces a clone of the key regardless of whether the entry exists.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to significantly improve performance by only cloning the key when it's actually being inserted.
