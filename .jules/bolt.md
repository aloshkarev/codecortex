## 2026-05-09 - Optimize HashMap::entry in hot loops
**Learning:** In Rust, avoiding `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) prevents unnecessary memory allocation on every iteration.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to significantly improve performance.
