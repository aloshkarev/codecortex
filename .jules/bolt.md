## 2024-05-03 - [HashMap Allocation Overhead]
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths causes unnecessary memory allocation on every iteration, as `key.clone()` is evaluated eagerly regardless of whether the key exists.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` on performance critical code like TF-IDF or metrics.
