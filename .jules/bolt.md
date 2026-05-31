## 2024-05-31 - [Avoid unnecesary HashMap String allocations on hot paths]
**Learning:** Using `*map.entry(key.clone()).or_insert(0) += 1` inside hot loops like TF-IDF term parsing creates an unnecessary `String` clone (allocation) for *every* item, even if the key already exists in the map. This is a common Rust performance anti-pattern.
**Action:** Replace the pattern with `if let Some(count) = map.get_mut(key) { *count += 1 } else { map.insert(key.clone(), 1) }`. This only clones the string when inserting a completely new item, avoiding thousands of allocations during text processing.
