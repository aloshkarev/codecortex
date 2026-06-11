//! Graph bulk write preparation benchmarks (CPU-side node batching).
//!
//! Measures synthetic node construction and writer chunking without requiring FalkorDB.
//! For live Bolt throughput use `RUN_DOCKER_INTEGRATION=1 cargo test -p cortex-graph --test falkordb_bulk_throughput_test`.

use cortex_core::{CodeNode, EntityKind, Language};
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::collections::HashMap;

fn synthetic_nodes(n: usize, with_source: bool) -> Vec<CodeNode> {
    (0..n)
        .map(|i| {
            let mut props = HashMap::new();
            props.insert("repository_path".to_string(), "/bench".to_string());
            props.insert("branch".to_string(), "main".to_string());
            CodeNode {
                id: format!("fn:bench:{i}"),
                kind: EntityKind::Function,
                name: format!("f{i}"),
                path: Some("bench.rs".to_string()),
                line_number: Some(i as u32),
                lang: Some(Language::Rust),
                source: if with_source {
                    Some("x".repeat(512))
                } else {
                    None
                },
                docstring: None,
                properties: props,
            }
        })
        .collect()
}

fn bench_node_batch_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_bulk_node_build");
    for n in [256, 1024, 4096, 8192] {
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::new("no_source", n), &n, |b, &n| {
            b.iter(|| black_box(synthetic_nodes(n, false)));
        });
        group.bench_with_input(BenchmarkId::new("with_source", n), &n, |b, &n| {
            b.iter(|| black_box(synthetic_nodes(n, true)));
        });
    }
    group.finish();
}

fn bench_node_chunk_split(c: &mut Criterion) {
    let nodes = synthetic_nodes(8192, false);
    let mut group = c.benchmark_group("graph_bulk_chunk_split");
    for chunk in [256, 512, 1024, 4096] {
        group.bench_with_input(BenchmarkId::new("chunks", chunk), &chunk, |b, &chunk| {
            b.iter(|| {
                let chunks: Vec<&[CodeNode]> = nodes.chunks(chunk).collect();
                black_box(chunks.len())
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_node_batch_build, bench_node_chunk_split);
criterion_main!(benches);
