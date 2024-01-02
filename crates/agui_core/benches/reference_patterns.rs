use std::{cell::RefCell, rc::Rc};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustc_hash::FxHashMap;

thread_local! {
    static THREAD_LOCAL: RefCell<FxHashMap<usize, usize>> = RefCell::default();
}

fn reference_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("reference patterns");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(1000).bench_function("map key", |b| {
        b.iter_with_setup(
            || {
                let mut map = FxHashMap::default();

                for i in 0..100 {
                    map.insert(i, i);
                }

                map
            },
            |map| *black_box(map.get(&black_box(37)).unwrap()),
        )
    });

    group.sample_size(1000).bench_function("rc", |b| {
        b.iter_with_setup(
            || Rc::new(0),
            |value| *black_box(Rc::clone(&black_box(value))),
        )
    });

    THREAD_LOCAL.with(|value| value.borrow_mut().insert(0, 0));

    group.sample_size(1000).bench_function("thread local", |b| {
        b.iter(|| THREAD_LOCAL.with(|value| black_box(*value.borrow().get(&black_box(0)).unwrap())))
    });

    group.finish();
}

criterion_group!(benches, reference_patterns);
criterion_main!(benches);
