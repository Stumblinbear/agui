use criterion::{criterion_group, criterion_main, Criterion};

fn channels(c: &mut Criterion) {
    let mut group = c.benchmark_group("channels (no receivers)");

    group.throughput(criterion::Throughput::Elements(1));

    group
        .sample_size(1000)
        .bench_function("std", |b| b.iter_with_setup(|| (), |_| {}));

    group.finish();
}

criterion_group!(benches, channels);
criterion_main!(benches);
