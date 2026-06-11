use cortex_pipeline::{Pipeline, PipelineContext};
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_pipeline_default_stages_build(c: &mut Criterion) {
    c.bench_function("pipeline_with_default_stages_construct", |b| {
        b.iter(|| {
            let _ = Pipeline::with_default_stages();
        })
    });
}

fn bench_pipeline_empty_run(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let pipeline = Pipeline::new();
    c.bench_function("pipeline_empty_run", |b| {
        b.iter(|| {
            let ctx = PipelineContext::from_content(
                "bench.rs".to_string(),
                "fn main() {}".to_string(),
                Some("rust".to_string()),
            );
            rt.block_on(async {
                let _ = pipeline.run(ctx).await;
            })
        })
    });
}

criterion_group!(
    benches,
    bench_pipeline_default_stages_build,
    bench_pipeline_empty_run
);
criterion_main!(benches);
