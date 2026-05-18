## 2024-05-18 - Avoid entry() with clone() in hot paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot paths (like TF-IDF term frequency counters) causes unnecessary memory allocation on every iteration because `clone()` is evaluated before the entry is checked.
**Action:** Use `if let Some(count) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` instead to significantly improve performance by only cloning strings on new insertions.
