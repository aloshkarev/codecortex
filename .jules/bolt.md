## 2024-05-13 - [HashMap Entry Allocation]
**Learning:** In Rust, avoid using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) as it causes unnecessary memory allocation on every iteration.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead. Note: When querying a `HashMap<String, V>` using a borrowed key from a `HashSet<&String>`, use `.get_mut(term.as_str())` to avoid type mismatch errors.
