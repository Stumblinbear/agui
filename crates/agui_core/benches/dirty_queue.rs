use std::{
    hash::BuildHasherDefault,
    sync::{Arc, Barrier},
};

use agui_core::element::ElementId;
use criterion::{criterion_group, criterion_main, Bencher, Criterion};

use parking_lot::Mutex;
use rustc_hash::FxHasher;
use slotmap::KeyData;

fn dirty_queue(c: &mut Criterion) {
    const SAMPLE_SIZE: usize = 20;

    const NUM_THREADS: usize = 1;
    const NUM_ELEMENTS: u64 = 500;

    let mut group = c.benchmark_group("dirty queue");

    group.throughput(criterion::Throughput::Elements(
        NUM_THREADS as u64 * NUM_ELEMENTS,
    ));

    fn do_bench<Q, QF, PF, IF>(
        b: &mut Bencher<'_>,
        elements: Arc<[ElementId]>,
        queue_fn: QF,
        process_func: PF,
        iter_fn: IF,
    ) where
        Q: Clone + Send + Sync + 'static,
        QF: Fn() -> Q,
        PF: Fn(Q, &[ElementId]) + Clone + Send + 'static,
        IF: Fn(Q, &[ElementId]),
    {
        if NUM_THREADS > 1 {
            b.iter_with_setup(
                || {
                    let queue = queue_fn();

                    let start_processing = Arc::new(Barrier::new(NUM_THREADS + 1));

                    let threads = (0..NUM_THREADS)
                        .map(|_| {
                            std::thread::spawn({
                                let elements = Arc::clone(&elements);

                                let start_processing = start_processing.clone();

                                let queue = queue.clone();
                                let process_func = process_func.clone();

                                move || {
                                    start_processing.wait();

                                    process_func(queue, &elements);
                                }
                            })
                        })
                        .collect::<Vec<_>>();

                    (queue, start_processing, threads)
                },
                |(queue, start_processing, threads)| {
                    start_processing.wait();

                    for handle in threads {
                        handle.join().expect("failed to join thread");
                    }

                    iter_fn(queue, &elements);
                },
            )
        } else {
            b.iter_with_setup(queue_fn, |queue| {
                let elements = Arc::clone(&elements);

                let queue = queue.clone();
                let process_func = process_func.clone();

                process_func(queue.clone(), &elements);

                iter_fn(queue, &elements);
            })
        }
    }

    let elements = (0..NUM_ELEMENTS)
        .map(|i| ElementId::from(KeyData::from_ffi(i)))
        .collect::<Arc<[ElementId]>>();

    group
        .sample_size(SAMPLE_SIZE)
        .bench_function("concurrent_queue", |b| {
            let elements = elements.clone();

            do_bench(
                b,
                elements,
                || Arc::new(concurrent_queue::ConcurrentQueue::<ElementId>::unbounded()),
                move |queue, elements| {
                    for element_id in elements.iter() {
                        queue.push(*element_id).expect("failed to push element");
                    }
                },
                move |queue, elements| {
                    let mut deduplicate_map = slotmap::SparseSecondaryMap::<
                        ElementId,
                        (),
                        BuildHasherDefault<FxHasher>,
                    >::default();

                    for element_id in queue.try_iter() {
                        if deduplicate_map.insert(element_id, ()).is_some() {
                            continue;
                        }

                        assert!(elements.contains(&element_id));
                    }
                },
            )
        });

    group.sample_size(SAMPLE_SIZE).bench_function("mutex", |b| {
        let elements = elements.clone();

        do_bench(
            b,
            elements,
            || {
                Arc::new(Mutex::new(slotmap::SparseSecondaryMap::<
                    ElementId,
                    (),
                    BuildHasherDefault<FxHasher>,
                >::default()))
            },
            move |queue, elements| {
                for element_id in elements.iter() {
                    queue.lock().insert(*element_id, ());
                }
            },
            move |queue, elements| {
                for element_id in queue.lock().drain().map(|(element_id, _)| element_id) {
                    assert!(elements.contains(&element_id));
                }
            },
        )
    });

    group.finish();
}

criterion_group!(benches, dirty_queue);
criterion_main!(benches);
