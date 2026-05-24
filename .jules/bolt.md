## 2024-05-24 - Avoid unnecessary memory allocation in hot loops with HashMap
**Learning:** In Rust, using `HashMap::entry(key.clone()).or_insert(...)` in hot loops (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration, as `clone()` is evaluated before the entry is retrieved.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to significantly improve performance on hot paths.
