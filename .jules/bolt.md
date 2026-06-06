## 2024-05-18 - Avoid entry().or_insert() on hot paths

**Learning:** `HashMap::entry(key.clone()).or_insert(...)` on hot paths like TF-IDF frequency counters causes unnecessary heap allocations for keys on every iteration, even when the key already exists.
**Action:** Use `.get_mut()` check first, then fallback to `.insert()` for the fast path, or rely on `.as_str()` if searching with a string reference.
