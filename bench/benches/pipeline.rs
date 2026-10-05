//! 계획서 8장의 지연 예산을 검증한다.
//! - 특징 갱신: < 50µs / event
//! - 점수 산출: < 1ms

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use guard_bench::synthetic_session;
use guard_collector::RingBuffer;
use guard_features::FeatureExtractor;
use guard_scorer::Scorer;

fn bench_pipeline(c: &mut Criterion) {
    let events = synthetic_session(10_000);

    c.bench_function("ring_push", |b| {
        let mut rb = RingBuffer::with_capacity(8192);
        let e = events[0];
        b.iter(|| rb.push(black_box(e)));
    });

    c.bench_function("feature_update_per_event", |b| {
        let mut fx = FeatureExtractor::new();
        let mut i = 0;
        b.iter(|| {
            fx.update(black_box(&events[i % events.len()]));
            i += 1;
        });
    });

    let mut fx = FeatureExtractor::new();
    events.iter().for_each(|e| fx.update(e));
    let scorer = Scorer::default();
    c.bench_function("snapshot_and_score", |b| {
        b.iter(|| scorer.score(black_box(&fx.snapshot()), true));
    });
}

criterion_group!(benches, bench_pipeline);
criterion_main!(benches);
