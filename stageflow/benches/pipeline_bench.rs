//! Benchmarks for pipeline execution.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn pipeline_benchmark(c: &mut Criterion) {
    c.bench_function("noop", |b| {
        b.iter(|| {
            black_box(42)
        })
    });
}

criterion_group!(benches, pipeline_benchmark);
criterion_main!(benches);
