use criterion::measurement::WallTime;
use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};

use agui_core::listenable::{Event, EventBus, EventEmitter, Listenable, Notifier, ValueNotifier};

struct TestEvent;

impl Event for TestEvent {}

fn events(c: &mut Criterion) {
    let mut group = c.benchmark_group("notifier");

    bench_events(
        &mut group,
        Notifier::new,
        |notifier| notifier.add_listener(|| {}),
        |notifier| notifier.notify_listeners(),
    );

    group.finish();

    let mut group = c.benchmark_group("emitter");

    bench_events(
        &mut group,
        EventEmitter::<TestEvent>::new,
        |emitter| emitter.add_listener(|_| {}),
        |emitter| emitter.emit(&TestEvent),
    );

    group.finish();

    let mut group = c.benchmark_group("value notifier");

    bench_events(
        &mut group,
        || ValueNotifier::<TestEvent>::new(TestEvent),
        |notifier| notifier.add_listener(|| {}),
        |notifier| notifier.set(TestEvent),
    );

    group.finish();

    let mut group = c.benchmark_group("event bus");

    bench_events(
        &mut group,
        EventBus::new,
        |bus| bus.add_listener::<TestEvent>(|_| {}),
        |bus| bus.emit(&TestEvent),
    );

    group.finish();

    // let mut group = c.benchmark_group("events");
    //
    // group.throughput(criterion::Throughput::Elements(1));

    // group.sample_size(500).bench_function("bus", |b| {
    //     b.iter_with_setup(
    //         || {
    //             let emitter = EventEmitter::<usize>::new();
    //
    //             let handle = emitter.add_listener(|_| {});
    //
    //             (emitter, handle)
    //         },
    //         |(mut emitter, _)| {
    //             emitter.emit(&0);
    //         },
    //     )
    // });

    // group.finish();
}

fn bench_events<T, E>(
    group: &mut BenchmarkGroup<WallTime>,
    init_fn: fn() -> T,
    listen_fn: fn(&T) -> E,
    emit_fn: fn(T) -> (),
) {
    group
        .throughput(criterion::Throughput::Elements(1))
        .sample_size(500)
        .bench_function("emitting (1 listener)", |b| {
            #[allow(clippy::disallowed_types)]
            b.iter_with_setup(
                || {
                    let value = init_fn();

                    let handles = vec![listen_fn(&value)];

                    (value, handles)
                },
                |(value, _)| emit_fn(value),
            )
        });

    group
        .throughput(criterion::Throughput::Elements(100))
        .sample_size(500)
        .bench_function("emitting (100 listener)", |b| {
            #[allow(clippy::disallowed_types)]
            b.iter_with_setup(
                || {
                    let value = init_fn();

                    let mut handles = vec![];

                    for _ in 0..100 {
                        handles.push(listen_fn(&value));
                    }

                    (value, handles)
                },
                |(value, _)| emit_fn(value),
            )
        });

    group
        .throughput(criterion::Throughput::Elements(1000))
        .sample_size(500)
        .bench_function("emitting (1000 listener)", |b| {
            #[allow(clippy::disallowed_types)]
            b.iter_with_setup(
                || {
                    let value = init_fn();

                    let mut handles = vec![];

                    for _ in 0..1000 {
                        handles.push(listen_fn(&value));
                    }

                    (value, handles)
                },
                |(value, _)| emit_fn(value),
            )
        });
}

criterion_group!(benches, events);
criterion_main!(benches);
