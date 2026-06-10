
## 2024-05-24 - Remove HashMap::entry().or_insert() on hot paths
**Learning:** Using `HashMap::entry(key.clone()).or_insert(...)` on hot loops (such as TF-IDF scoring or term frequencies) causes unnecessary memory allocation on every iteration, leading to significant overhead in large collections.
**Action:** Always prefer using `if let Some(val) = map.get_mut(key)` with a fallback to `insert(key.clone(), ...)` on highly repetitive hot paths where the keys are borrowed. Note that for `HashSet<&String>`, querying a `HashMap<String, V>` requires using `.get_mut(term.as_str())` to avoid type mismatch errors.
