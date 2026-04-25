## 2025-04-25 - Avoid String Clones in TF-IDF Term Frequency Counters
**Learning:** In Rust's `HashMap::entry(key).or_insert(...)` pattern, if `key` is an owned `String` that is cloned each time (e.g. `entry(term.clone())`), we incur the cost of memory allocation on every iteration, regardless of whether the key already exists.
**Action:** Replace the `entry` pattern with `if let Some(count) = map.get_mut(key)` for hot paths, and fallback to `insert(key.clone(), ...)` to avoid allocating a new string when incrementing counters for existing terms.
