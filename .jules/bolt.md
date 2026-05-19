## 2024-05-19 - Avoid HashMap::entry().or_insert() on hot paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths like TF-IDF term frequency counters causes unnecessary memory allocation on every iteration because the key is cloned even if it already exists in the map.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `map.insert(key.clone(), ...)` instead. This significantly improves performance by avoiding the redundant clone operations.
