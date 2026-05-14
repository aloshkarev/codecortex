## 2024-05-14 - HashMap Entry Pattern Causes Memory Allocation Overhead on Hot Paths
**Learning:** In Rust, using `HashMap::entry(key.clone()).or_insert(...)` within loops or hot paths (like TF-IDF term frequency calculations) causes unnecessary memory allocation on every single iteration because `clone()` executes before checking if the key exists. This is a common pattern that impacts performance significantly on large iterations.
**Action:** Instead, use a combination of `get_mut` to update existing entries and `insert` as a fallback:
```rust
if let Some(count) = map.get_mut(key) {
    *count += 1.0;
} else {
    map.insert(key.clone(), 1.0);
}
```
This avoids cloning the key if it already exists in the map.
