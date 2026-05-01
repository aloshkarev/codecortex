## 2024-05-01 - Avoid `HashMap::entry` on hot paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration, as `.clone()` runs before `entry()` resolves, even if the key is already in the map.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to eliminate this unnecessary string allocation and significantly improve performance.
