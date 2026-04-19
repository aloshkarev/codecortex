
## 2024-04-19 - Replace JSON serialization with in-memory downcasting for L1 cache
**Learning:** For in-memory caching mechanisms in Rust, using `Box<dyn std::any::Any + Send + Sync>` combined with downcasting instead of byte serialization strategies (like `serde_json`) significantly reduces overhead and improves performance.
**Action:** When creating in-memory caches, prefer utilizing `Any` and downcasting over serialization to minimize CPU usage and improve throughput.
