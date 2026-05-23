
## 2024-05-23 - Optimize HashMap Entry Usage on Hot Paths
**Learning:** `HashMap::entry(key.clone()).or_insert(...)` is a common anti-pattern on hot paths because it forces a memory allocation and string copy for every single iteration, even when the key already exists. This can be a major bottleneck in text processing algorithms like TF-IDF where term frequencies have a long-tail distribution (same words appear many times).
**Action:** Always check if a key exists first using `get_mut` when iterating over collections where elements frequently repeat, and only use `clone` and `insert` as a fallback.
