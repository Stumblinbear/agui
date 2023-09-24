use agui_core::widget::IntoWidget;
use agui_primitives::sized_box::SizedBox;
use criterion::{criterion_group, criterion_main, Criterion};

use agui::{engine::Engine, widgets::primitives::flex::Column};

fn engine_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine (single)");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(500).bench_function("additions", |b| {
        b.iter_with_setup(
            || (Engine::new(), Column::builder().build()),
            |(mut engine, widget)| {
                engine.set_root(widget);

                engine.update();
            },
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let mut engine = Engine::with_root(Column::builder().build());

                engine.update();

                engine
            },
            |mut engine| {
                engine.remove_root();

                engine.update();
            },
        )
    });

    group.finish();

    let mut group = c.benchmark_group("engine (large)");

    group.throughput(criterion::Throughput::Elements(1000));

    group.sample_size(500).bench_function("additions", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::builder().build();

                for _ in 0..1000 {
                    column
                        .children
                        .push(SizedBox::builder().build().into_widget().into());
                }

                (Engine::new(), column)
            },
            |(mut engine, widget)| {
                engine.set_root(widget);

                engine.update();
            },
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::builder().build();

                for _ in 0..1000 {
                    column
                        .children
                        .push(SizedBox::builder().build().into_widget().into());
                }

                let mut engine = Engine::with_root(column);

                engine.update();

                engine
            },
            |mut engine| {
                engine.remove_root();

                engine.update();
            },
        )
    });

    group.finish();
}

criterion_group!(benches, engine_ops);
criterion_main!(benches);
