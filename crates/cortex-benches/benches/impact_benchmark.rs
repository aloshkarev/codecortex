//! Benchmarks for Impact Graph Builder
//!
//! Measures performance of:
//! - Impact graph building with various caller counts
//! - Blast radius classification
//! - Edge traversal

use cortex_mcp::{ImpactGraphBuilder, Provenance, RawRelation};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

fn make_relation(from_id: &str, from_name: &str, to_id: &str) -> RawRelation {
    RawRelation {
        from_id: from_id.to_string(),
        from_name: from_name.to_string(),
        from_path: Some(format!("/src/{}.rs", from_name)),
        to_id: to_id.to_string(),
        relation_type: "calls".to_string(),
        confidence: 0.9,
        provenance: Provenance::Static,
    }
}

fn generate_callers(count: usize) -> Vec<RawRelation> {
    (0..count)
        .map(|i| make_relation(&format!("func:{}", i), &format!("caller_{}", i), "target"))
        .collect()
}

fn generate_transitive_chain(depth: usize) -> (Vec<RawRelation>, Vec<RawRelation>) {
    let direct = vec![make_relation("direct", "direct_caller", "target")];

    let transitive: Vec<RawRelation> = (0..depth)
        .map(|i| {
            make_relation(
                &format!("trans_{}", i),
                &format!("transitive_{}", i),
                "target",
            )
        })
        .collect();

    (direct, transitive)
}

fn bench_impact_graph_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("impact_graph_build");

    for size in [10, 50, 100, 500, 1000].iter() {
        let direct = generate_callers(*size);
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::new("build", size), size, |b, _| {
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

    group.finish();
}

fn bench_impact_with_transitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("impact_transitive");

    for depth in [5, 10, 20, 50, 100].iter() {
        let (direct, transitive) = generate_transitive_chain(*depth);
        group.throughput(Throughput::Elements(*depth as u64));

        group.bench_with_input(BenchmarkId::new("depth", depth), depth, |b, _| {
            b.iter(|| {
                let builder = ImpactGraphBuilder::new();
                let result = builder.build(
                    black_box("target"),
                    black_box(None),
                    black_box(None),
                    black_box(direct.clone()),
                    black_box(transitive.clone()),
                    black_box(vec![]),
                    black_box(vec![]),
                );
                black_box(result)
            });
        });
    }

    group.finish();
}

fn bench_blast_radius_classification(c: &mut Criterion) {
    let mut group = c.benchmark_group("blast_radius");

    // Low blast radius (5 callers)
    let low = generate_callers(5);
    group.bench_function("low_blast_radius", |b| {
        b.iter(|| {
            let builder = ImpactGraphBuilder::new();
            let result = builder.build(
                black_box("target"),
                black_box(None),
                black_box(None),
                black_box(low.clone()),
                black_box(vec![]),
                black_box(vec![]),
                black_box(vec![]),
            );
            black_box(result)
        });
    });

    // Medium blast radius (15 callers)
    let medium = generate_callers(15);
    group.bench_function("medium_blast_radius", |b| {
        b.iter(|| {
            let builder = ImpactGraphBuilder::new();
            let result = builder.build(
                black_box("target"),
                black_box(None),
                black_box(None),
                black_box(medium.clone()),
                black_box(vec![]),
                black_box(vec![]),
                black_box(vec![]),
            );
            black_box(result)
        });
    });

    // High blast radius (50 callers)
    let high = generate_callers(50);
    group.bench_function("high_blast_radius", |b| {
        b.iter(|| {
            let builder = ImpactGraphBuilder::new();
            let result = builder.build(
                black_box("target"),
                black_box(None),
                black_box(None),
                black_box(high.clone()),
                black_box(vec![]),
                black_box(vec![]),
                black_box(vec![]),
            );
            black_box(result)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_impact_graph_build,
    bench_impact_with_transitive,
    bench_blast_radius_classification,
);

criterion_main!(benches);
