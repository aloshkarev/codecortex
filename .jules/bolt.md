## 2026-05-10 - HashMap Allocation Bottleneck in Rust
**Learning:** In Rust, avoiding `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) is critical, as it causes unnecessary memory allocation on every single iteration due to the `.clone()`.
**Action:** Use `if let Some(count) = map.get_mut(key) { *count += 1 } else { map.insert(key.clone(), 1) }` to significantly improve performance by only allocating when the key doesn't exist yet.
