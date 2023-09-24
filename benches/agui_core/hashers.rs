use std::any::TypeId;

use agui_core::{callback::Callback, element::ElementId};
use criterion::{criterion_group, criterion_main, Criterion};

use slotmap::KeyData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct CallbackId {
    element_id: ElementId,
    type_id: TypeId,
}

fn hashers(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashers (8 bytes)");

    group.throughput(criterion::Throughput::Elements(1));

    let element_id = ElementId::from(KeyData::from_ffi(0));

    group.sample_size(500).bench_function("std", |b| {
        #[allow(clippy::disallowed_types)]
        b.iter_with_setup(
            std::collections::HashMap::<ElementId, ()>::default,
            |mut map| {
                map.insert(element_id, ());
            },
        )
    });

    group.sample_size(500).bench_function("fx", |b| {
        b.iter_with_setup(
            rustc_hash::FxHashMap::<ElementId, ()>::default,
            |mut map| {
                map.insert(element_id, ());
            },
        )
    });

    group.sample_size(500).bench_function("fnv", |b| {
        b.iter_with_setup(fnv::FnvHashMap::<ElementId, ()>::default, |mut map| {
            map.insert(element_id, ());
        })
    });

    group.finish();

    let mut group = c.benchmark_group("hashers (16 bytes)");

    group.throughput(criterion::Throughput::Elements(1));

    let callback_id = CallbackId {
        element_id,
        type_id: TypeId::of::<Callback<()>>(),
    };

    group.sample_size(500).bench_function("std", |b| {
        #[allow(clippy::disallowed_types)]
        b.iter_with_setup(
            std::collections::HashMap::<CallbackId, ()>::default,
            |mut map| {
                map.insert(callback_id, ());
            },
        )
    });

    group.sample_size(500).bench_function("fx", |b| {
        b.iter_with_setup(
            rustc_hash::FxHashMap::<CallbackId, ()>::default,
            |mut map| {
                map.insert(callback_id, ());
            },
        )
    });

    group.sample_size(500).bench_function("fnv", |b| {
        b.iter_with_setup(fnv::FnvHashMap::<CallbackId, ()>::default, |mut map| {
            map.insert(callback_id, ());
        })
    });

    group.finish();
}

criterion_group!(benches, hashers);
criterion_main!(benches);
