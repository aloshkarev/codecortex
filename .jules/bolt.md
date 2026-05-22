## 2024-05-22 - HashMap entry allocation optimization
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` causes unnecessary String allocations on every iteration even when the key exists, which can be a significant bottleneck in hot paths like TF-IDF frequency counters.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback `insert(key.clone(), ...)` instead to only allocate when a new entry is actually created.
