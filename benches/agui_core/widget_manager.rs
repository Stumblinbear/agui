use criterion::{criterion_group, criterion_main, Criterion};

use agui::{manager::WidgetManager, widgets::primitives::flex::Column};

fn widget_manager_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("widget manager (single)");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(500).bench_function("additions", |b| {
        b.iter_with_setup(
            || (WidgetManager::new(), Column::builder().build()),
            |(mut manager, widget)| {
                manager.set_root(widget);

                manager.update();
            },
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let mut manager = WidgetManager::with_root(Column::builder().build());

                manager.update();

                manager
            },
            |mut manager| {
                manager.remove_root();

                manager.update();
            },
        )
    });

    group.finish();

    let mut group = c.benchmark_group("widget manager (large)");

    group.throughput(criterion::Throughput::Elements(1000));

    group.sample_size(500).bench_function("additions", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::builder().build();

                for _ in 0..1000 {
                    column.children.push(Column::builder().build().into());
                }

                (WidgetManager::new(), column)
            },
            |(mut manager, widget)| {
                manager.set_root(widget);

                manager.update();
            },
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::builder().build();

                for _ in 0..1000 {
                    column.children.push(Column::builder().build().into());
                }

                let mut manager = WidgetManager::with_root(column);

                manager.update();

                manager
            },
            |mut manager| {
                manager.remove_root();

                manager.update();
            },
        )
    });

    group.finish();
}

criterion_group!(benches, widget_manager_ops);
criterion_main!(benches);
