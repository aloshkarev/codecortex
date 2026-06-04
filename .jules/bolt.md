## 2024-05-30 - Map entry optimization on hot paths
**Learning:** `HashMap::entry(key.clone()).or_insert(...)` can cause high unnecessary memory allocation overhead in frequently executed hot paths (like TF-IDF term counters and document frequencies) because it unconditionally clones the key before even checking if it exists in the map.
**Action:** Use the pattern `if let Some(val) = map.get_mut(key) { *val += 1; } else { map.insert(key.clone(), 1); }` for hot paths to significantly avoid redundant allocations and improve performance.
