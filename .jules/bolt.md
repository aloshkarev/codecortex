## 2025-01-20 - HashMap Entry API Cloning Overhead
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration because the key is cloned before checking if it exists.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` to avoid allocating when the key is already in the map.
