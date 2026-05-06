## 2024-05-06 - Initial setup
**Learning:** Found an instruction in memory: "In Rust, avoid using HashMap::entry(key.clone()).or_insert(...) on hot paths (like TF-IDF term frequency counters) as it causes unnecessary memory allocation on every iteration. Use if let Some(count) = map.get_mut(key) with a fallback to insert(key.clone(), ...) instead to significantly improve performance."
**Action:** Replace `HashMap::entry(key.clone()).or_insert(...)` with `if let Some(count) = map.get_mut(key)` in `cortex-mcp/src/tfidf.rs` and other performance critical spots.
