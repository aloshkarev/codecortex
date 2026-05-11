## 2024-05-11 - Fast hashmap entry pattern
**Learning:** In Rust, avoiding `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) significantly improves performance by reducing memory allocation per iteration.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead.
