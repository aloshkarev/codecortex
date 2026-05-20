## 2024-05-20 - Avoid unnecessary String allocation in TF-IDF Map inserts
**Learning:** In hot loops like `term_frequency` and document indexing in `tfidf.rs`, doing `HashMap::entry(key.clone()).or_insert(...)` requires allocating a new String for the cloned key on every lookup, even if the key is already present. This results in heavy heap allocations and poor performance.
**Action:** Use `if let Some(val) = map.get_mut(key)` followed by an `else { map.insert(key.clone(), ...) }` when the key is a string to prevent the unnecessary string allocations on successful lookups.

## 2024-05-20 - Protobuf cache must be cleaned up
**Learning:** The development environment uses `protoc` downloaded as a temporary artifact for compilation. If these artifacts are left around, they will be erroneously checked in with `git add .` unless strictly cleaned up or ignored.
**Action:** Always delete temporary build artifacts like `protoc` zips, extracted directories, and `.diff` files before submitting.
