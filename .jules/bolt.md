## 2026-05-30 - [Bolt: HashMap avoid key clone allocation]
**Learning:** In Rust, avoid using `HashMap::entry(key.clone()).or_insert(...)` on hot paths as it causes unnecessary memory allocation on every iteration. Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to significantly improve performance.
**Action:** Always check map insertion patterns in hot loops (like TF-IDF term frequencies) and optimize away clone allocations.
