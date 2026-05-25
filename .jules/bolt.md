## 2026-05-25 - HashMap Entry API Anti-Pattern on Hot Paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration because `clone()` executes before the entry's existence is checked. This introduces significant string allocation overhead.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to eliminate this allocation overhead and improve performance.
