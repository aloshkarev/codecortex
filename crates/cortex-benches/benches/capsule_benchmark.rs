//! Benchmarks for Context Capsule Builder
//!
//! Measures performance of:
//! - Capsule building with various corpus sizes
//! - Scoring operations
//! - Threshold relaxation

use cortex_mcp::{CapsuleConfig, ContextCapsuleBuilder, GraphSearchResult};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

fn make_result(id: &str, name: &str, path: &str, source: &str) -> GraphSearchResult {
    GraphSearchResult {
        id: id.to_string(),
        kind: "Function".to_string(),
        path: path.to_string(),
        name: name.to_string(),
        source: Some(source.to_string()),
        line_number: Some(1),
    }
}

fn generate_corpus(size: usize) -> Vec<GraphSearchResult> {
    (0..size)
        .map(|i| {
            make_result(
                &format!("func:{}", i),
                &format!("function_{}", i),
                &format!("/src/module{}/file{}.rs", i % 10, i % 100),
                &format!(
                    "pub fn function_{}(arg: &str) -> Result<(), Error> {{ /* implementation */ }}",
                    i
                ),
            )
        })
        .collect()
}

fn bench_capsule_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("capsule_build");

    for size in [10, 50, 100, 500, 1000].iter() {
        let corpus = generate_corpus(*size);
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("build", size), size, |b, _| {
            b.iter(|| {
                let mut builder = ContextCapsuleBuilder::new();
                let result = builder.build(
                    black_box("function"),
                    black_box(corpus.clone()),
                    black_box(None),
                    black_box(&[]),
                );
                black_box(result)
            });
        });
    }

    group.finish();
}

fn bench_capsule_with_intent(c: &mut Criterion) {
    let mut group = c.benchmark_group("capsule_intent");
    let corpus = generate_corpus(100);

    for intent in ["debug", "refactor", "explore", "test"].iter() {
        group.bench_with_input(BenchmarkId::new("intent", intent), intent, |b, intent| {
            b.iter(|| {
                let mut builder = ContextCapsuleBuilder::new().with_intent(intent);
                let result = builder.build(
                    black_box("function"),
                    black_box(corpus.clone()),
                    black_box(None),
                    black_box(&[]),
                );
                black_box(result)
            });
        });
    }

    group.finish();
}

fn bench_capsule_threshold_relaxation(c: &mut Criterion) {
    let mut group = c.benchmark_group("capsule_threshold");
    let corpus = generate_corpus(100);

    // High threshold - will need relaxation
    let config_strict = CapsuleConfig {
        initial_threshold: 0.9,
        min_threshold: 0.1,
        relaxation_step: 0.1,
        ..Default::default()
    };

    // Low threshold - no relaxation needed
    let config_relaxed = CapsuleConfig {
        initial_threshold: 0.1,
        min_threshold: 0.05,
        relaxation_step: 0.02,
        ..Default::default()
    };

    group.bench_function("strict_threshold", |b| {
        b.iter(|| {
            let mut builder = ContextCapsuleBuilder::with_config(config_strict.clone());
            let result = builder.build(
                black_box("function"),
                black_box(corpus.clone()),
                black_box(None),
                black_box(&[]),
            );
            black_box(result)
        });
    });

    group.bench_function("relaxed_threshold", |b| {
        b.iter(|| {
            let mut builder = ContextCapsuleBuilder::with_config(config_relaxed.clone());
            let result = builder.build(
                black_box("function"),
                black_box(corpus.clone()),
                black_box(None),
                black_box(&[]),
            );
            black_box(result)
        });
    });

    group.finish();
}

fn bench_lexical_scoring(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexical_scoring");

    let test_cases = vec![
        (
            "authenticate",
            "authenticate_user",
            "pub fn authenticate_user() {}",
        ),
        ("auth", "login", "pub fn login() {}"),
        (
            "database",
            "connect_to_database",
            "pub fn connect_to_database() {}",
        ),
        ("cache", "cache_result", "pub fn cache_result() {}"),
    ];

    for (query, name, source) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("match", query),
            &(query, name, source),
            |b, &(q, n, s)| {
                b.iter(|| {
                    let mut builder = ContextCapsuleBuilder::new();
                    let results = vec![make_result("test", n, "/src/test.rs", s)];
                    let result = builder.build(
                        black_box(q),
                        black_box(results),
                        black_box(None),
                        black_box(&[]),
                    );
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_capsule_build,
    bench_capsule_with_intent,
    bench_capsule_threshold_relaxation,
    bench_lexical_scoring,
);

criterion_main!(benches);
