use cortex_watcher::BoundedEventQueue;
use criterion::{Criterion, criterion_group, criterion_main};

fn bench_bounded_queue_saturation(c: &mut Criterion) {
    c.bench_function("bounded_event_queue_push_until_full", |b| {
        b.iter(|| {
            let mut q = BoundedEventQueue::new(1024, 1024 * 512);
            for i in 0u64..2048 {
                if q.push(i).is_err() {
                    break;
                }
            }
            q.clear();
        })
    });
}

criterion_group!(benches, bench_bounded_queue_saturation);
criterion_main!(benches);
