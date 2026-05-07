## 2026-05-07 - HashMap Entry API Cloning Cost
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths eagerly allocates memory for the clone, even if the key already exists. This creates significant CPU overhead due to unnecessary allocations.
**Action:** Use `if let Some(count) = map.get_mut(key) { ... } else { map.insert(key.clone(), ...); }` instead on performance-critical paths (like TF-IDF term frequency counters) to minimize memory allocations.
