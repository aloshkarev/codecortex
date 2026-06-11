//! Composite performance scenarios for CI regression gates.
//!
//! Runs representative micro-benchmarks across capsule building, impact graphs,
//! TF-IDF/BM25 scoring, and cache operations. Results are checked against
//! `perf_budget.json` by `scripts/measurement/check_perf_regression.py`.

use cortex_mcp::{
    CacheHierarchy, CapsuleConfig, ContextCapsuleBuilder, Document, GraphSearchResult,
    ImpactGraphBuilder, L1Cache, Provenance, RawRelation, TfIdfScorer,
};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn make_capsule_result(id: &str, name: &str, path: &str) -> GraphSearchResult {
    GraphSearchResult {
        id: id.to_string(),
        kind: "Function".to_string(),
        path: path.to_string(),
        name: name.to_string(),
        source: Some(format!("pub fn {name}() {{ /* body */ }}")),
        line_number: Some(1),
    }
}

fn make_relation(from_id: &str, from_name: &str) -> RawRelation {
    RawRelation {
        from_id: from_id.to_string(),
        from_name: from_name.to_string(),
        from_path: Some(format!("/src/{from_name}.rs")),
        to_id: "target".to_string(),
        relation_type: "calls".to_string(),
        confidence: 0.9,
        provenance: Provenance::Static,
    }
}

fn bench_scenario_capsule_build(c: &mut Criterion) {
    let corpus: Vec<_> = (0..100)
        .map(|i| {
            make_capsule_result(
                &format!("func:{i}"),
                &format!("function_{i}"),
                &format!("/src/module{i}/file.rs"),
            )
        })
        .collect();

    c.bench_function("scenario_capsule_build_100", |b| {
        b.iter(|| {
            let mut builder = ContextCapsuleBuilder::with_config(CapsuleConfig::default());
            let result = builder.build(
                black_box("authenticate"),
                black_box(corpus.clone()),
                black_box(None),
                black_box(&[]),
            );
            black_box(result)
        });
    });
}

fn bench_scenario_impact_graph(c: &mut Criterion) {
    let direct: Vec<_> = (0..50)
        .map(|i| make_relation(&format!("func:{i}"), &format!("caller_{i}")))
        .collect();

    c.bench_function("scenario_impact_graph_50", |b| {
        b.iter(|| {
            let builder = ImpactGraphBuilder::new();
            let result = builder.build(
                black_box("target"),
                black_box(None),
                black_box(None),
                black_box(direct.clone()),
                black_box(vec![]),
                black_box(vec![]),
                black_box(vec![]),
            );
            black_box(result)
        });
    });
}

fn bench_scenario_tfidf_score(c: &mut Criterion) {
    let docs: Vec<_> = (0..200)
        .map(|i| {
            Document::new(
                &format!("doc:{i}"),
                &format!(
                    "function authenticate user token refresh handler service module {i}"
                ),
            )
        })
        .collect();
    let scorer = TfIdfScorer::from_documents(&docs);

    c.bench_function("scenario_tfidf_score_200", |b| {
        b.iter(|| {
            let results = scorer.score_all(black_box("authenticate token"), black_box(&docs));
            black_box(results)
        });
    });
}

fn bench_scenario_cache_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("scenario_cache");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("l1_put_get_1000", |b| {
        let cache = L1Cache::new();
        b.iter(|| {
            for i in 0..1000 {
                let key = format!("key_{i}");
                cache.put(
                    black_box(key),
                    black_box(format!("value_{i}")),
                    black_box("rev1".to_string()),
                );
                black_box(cache.get::<String>(&key, "rev1"));
            }
        });
    });

    group.bench_function("hierarchy_put_get_500", |b| {
        let hierarchy = CacheHierarchy::new();
        b.iter(|| {
            for i in 0..500 {
                let key = format!("hk_{i}");
                hierarchy.put(
                    black_box(&key),
                    black_box(format!("hv_{i}")),
                    black_box("rev1".to_string()),
                );
                black_box(hierarchy.get::<String>(&key, "rev1"));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_scenario_capsule_build,
    bench_scenario_impact_graph,
    bench_scenario_tfidf_score,
    bench_scenario_cache_hierarchy,
);
criterion_main!(benches);
