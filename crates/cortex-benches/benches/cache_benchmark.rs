//! Benchmarks for Cache Hierarchy
//!
//! Measures performance of:
//! - L1 cache operations (get/put)
//! - L2 cache operations (disk-based)
//! - Cache hierarchy lookups

use cortex_mcp::{CacheHierarchy, L1Cache};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::time::Duration;

fn bench_l1_cache_put(c: &mut Criterion) {
    let mut group = c.benchmark_group("l1_cache_put");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("put", size), size, |b, _| {
            let cache = L1Cache::new();
            let data: Vec<(String, String)> = (0..*size)
                .map(|i| (format!("key_{}", i), format!("value_{}", i)))
                .collect();

            b.iter(|| {
                for (key, value) in &data {
                    cache.put(
                        black_box(key.clone()),
                        black_box(value.clone()),
                        black_box("rev1".to_string()),
                    );
                }
                black_box(())
            });
        });
    }

    group.finish();
}

fn bench_l1_cache_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("l1_cache_get");

    for size in [10, 100, 1000, 10000].iter() {
        let cache = L1Cache::new();

        // Pre-populate cache
        for i in 0..*size {
            cache.put(
                format!("key_{}", i),
                format!("value_{}", i),
                "rev1".to_string(),
            );
        }

        let keys: Vec<String> = (0..*size).map(|i| format!("key_{}", i)).collect();
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("get", size), size, |b, _| {
            b.iter(|| {
                for key in &keys {
                    let value: Option<String> = cache.get(black_box(key), black_box("rev1"));
                    black_box(value);
                }
            });
        });
    }

    group.finish();
}

fn bench_l1_cache_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("l1_cache_mixed");

    group.bench_function("get_put_mixed", |b| {
        let cache = L1Cache::new();

        // Pre-populate some data
        for i in 0..100 {
            cache.put(
                format!("key_{}", i),
                format!("value_{}", i),
                "rev1".to_string(),
            );
        }

        b.iter(|| {
            // Mix of gets and puts
            for i in 0..100 {
                if i % 3 == 0 {
                    let _: Option<String> =
                        cache.get(black_box(&format!("key_{}", i % 50)), black_box("rev1"));
                } else {
                    cache.put(
                        black_box(format!("key_{}", i)),
                        black_box(format!("value_{}", i)),
                        black_box("rev1".to_string()),
                    );
                }
            }
        });
    });

    group.finish();
}

fn bench_cache_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hierarchy");

    // Test L1 hit
    group.bench_function("l1_hit", |b| {
        let cache = CacheHierarchy::new();
        cache.put("test_key", "test_value".to_string(), "rev1".to_string());

        b.iter(|| {
            let (value, hit): (Option<String>, _) =
                cache.get(black_box("test_key"), black_box("rev1"));
            black_box((value, hit))
        });
    });

    // Test L2 hit (after L1 miss)
    group.bench_function("l2_hit", |b| {
        let cache = CacheHierarchy::new();
        cache.put("test_key", "test_value".to_string(), "rev1".to_string());

        // Clear L1 to force L2 lookup
        cache.l1().clear();

        b.iter(|| {
            let (value, hit): (Option<String>, _) =
                cache.get(black_box("test_key"), black_box("rev1"));
            black_box((value, hit))
        });
    });

    // Test miss
    group.bench_function("cache_miss", |b| {
        let cache = CacheHierarchy::new();

        b.iter(|| {
            let (value, hit): (Option<String>, _) =
                cache.get(black_box("nonexistent_key"), black_box("rev1"));
            black_box((value, hit))
        });
    });

    group.finish();
}

fn bench_cache_with_ttl(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_ttl");

    group.bench_function("put_with_ttl", |b| {
        let cache = L1Cache::new();

        b.iter(|| {
            cache.put_with_ttl(
                black_box("test_key".to_string()),
                black_box("test_value".to_string()),
                black_box(Duration::from_secs(30)),
                black_box("rev1".to_string()),
            );
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_l1_cache_put,
    bench_l1_cache_get,
    bench_l1_cache_mixed,
    bench_cache_hierarchy,
    bench_cache_with_ttl,
);

criterion_main!(benches);
