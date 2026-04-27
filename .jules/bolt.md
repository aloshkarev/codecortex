## 2024-06-25 - Avoid `HashMap::entry(key.clone())` on hot paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocations and string clones on every iteration, even when the key already exists in the map.
**Action:** Replace `entry(key.clone())` with `if let Some(val) = map.get_mut(key)` with a fallback to `map.insert(key.clone(), new_val)` to avoid cloning keys that are already present in the map, improving performance without altering the logic.
