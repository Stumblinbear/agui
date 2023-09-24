use std::{cell::RefCell, rc::Rc};

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rustc_hash::FxHashMap;

thread_local! {
    static THREAD_LOCAL: RefCell<FxHashMap<usize, usize>> = RefCell::default();
}

fn rc_vs_thread_local(c: &mut Criterion) {
    let mut group = c.benchmark_group("reference patterns");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(500).bench_function("rc", |b| {
        b.iter_with_setup(|| Rc::new(0), |value| black_box(*Rc::clone(&value)))
    });

    THREAD_LOCAL.with(|value| value.borrow_mut().insert(0, 0));

    group.sample_size(500).bench_function("thread local", |b| {
        b.iter(|| THREAD_LOCAL.with(|value| black_box(*value.borrow().get(&0).unwrap())))
    });

    group.finish();
}

criterion_group!(benches, rc_vs_thread_local);
criterion_main!(benches);
