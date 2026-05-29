## 2026-05-29 - Avoid HashMap::entry(key.clone()) on Hot Paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocations for every lookup, even if the key already exists.
**Action:** Use `if let Some(val) = map.get_mut(key)` with a fallback to `map.insert(key.clone(), ...)` instead to eliminate cloning allocations for existing entries.
