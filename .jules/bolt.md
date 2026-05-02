## 2024-05-18 - [TF-IDF String Allocation Optimization]
**Learning:** In a hot loop using `HashMap::entry(key.clone()).or_insert(0) += 1`, cloning the key on every iteration for lookup causes huge memory allocation overhead which hurts performance.
**Action:** Instead, do `if let Some(count) = map.get_mut(key) { *count += 1 } else { map.insert(key.clone(), 1) }` to avoid string cloning when the entry already exists.
