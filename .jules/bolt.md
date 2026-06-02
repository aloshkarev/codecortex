## 2024-05-18 - Avoid entry() and clone() in TF-IDF
**Learning:** In hot loops like `add_document` or `term_frequency` for TF-IDF calculations, using `map.entry(key.clone()).or_insert(...)` can cause unnecessary string clones even when the key already exists.
**Action:** Use `if let Some(count) = map.get_mut(term)` with `map.insert` as a fallback. For keys passed as `&String`, use `.as_str()` or deref instead of cloning.
