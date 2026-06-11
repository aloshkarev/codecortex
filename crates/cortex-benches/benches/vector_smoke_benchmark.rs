use cortex_vector::LanceStore;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;

fn bench_lance_open_temp(c: &mut Criterion) {
    c.bench_function("lance_store_open_temp_dir", |b| {
        b.iter_custom(|iters| {
            let mut total = std::time::Duration::ZERO;
            for _ in 0..iters {
                let dir = tempfile::tempdir().expect("tempdir");
                let path = PathBuf::from(dir.path());
                let start = std::time::Instant::now();
                let rt = tokio::runtime::Runtime::new().expect("runtime");
                rt.block_on(async {
                    let _store = LanceStore::open(&path).await.expect("open lance");
                });
                total += start.elapsed();
            }
            total
        })
    });
}

criterion_group!(benches, bench_lance_open_temp);
criterion_main!(benches);
