## 2024-04-21 - Optimizing `L1Cache` in `cortex-mcp`
**Learning:** `L1Cache` previously serialized values to JSON using `serde_json` and stored them as `Vec<u8>`. This added unnecessary serialization/deserialization overhead on every cache hit/miss for an *in-memory* cache.
**Action:** Replaced `Vec<u8>` with `Box<dyn Any + Send + Sync>` in the `DashMap` to enable true in-memory caching via downcasting, avoiding `serde_json` serialization and deserialization entirely for L1 caching.
