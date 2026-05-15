## 2024-05-24 - Avoid HashMap::entry().or_insert() on hot paths with String keys
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` inside hot loops (like TF-IDF term counters) causes an unnecessary String allocation on every single iteration, even for existing keys, resulting in significant memory churn and performance degradation.
**Action:** Replace this pattern with a two-step approach: first use `get_mut` to update existing values without allocating, then fall back to `insert(key.clone(), ...)` for new entries.
