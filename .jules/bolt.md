## 2024-05-16 - Avoid map.entry().or_insert() on hot paths in Rust
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration, leading to significant overhead in loops due to `clone()`.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to eliminate these allocations and significantly improve performance.
