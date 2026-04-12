## 2026-04-12 - Combining DashMaps in L1Cache
**Learning:** The L1 in-memory cache used two separate concurrent hash maps (DashMap) to store values and metadata separately. This resulted in redundant hashing and lock contention on every cache operation.
**Action:** Combined the value and metadata into a single struct (L1CacheEntry) stored in a single DashMap. Remember to structure concurrent caches to minimize the number of underlying concurrent map lookups per logical operation.
