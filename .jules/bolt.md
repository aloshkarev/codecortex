## 2026-06-12 - Avoiding redundant allocations in HashMap entry API
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` unconditionally clones the key on every lookup, even if the entry exists. In hot paths like TF-IDF or BM25 term frequency counting, this causes significant performance degradation due to memory allocations.
**Action:** When counting occurrences in a loop, avoid `entry` with a cloned key. Instead, use a lookup-first approach like `if let Some(v) = map.get_mut(key) { *v += 1.0; } else { map.insert(key.clone(), 1.0); }`.
