## 2024-05-24 - HashMap Entry API on Hot Paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration even when the key already exists.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to significantly improve performance on hot loops counting strings.
