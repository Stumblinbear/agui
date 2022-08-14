use criterion::{criterion_group, criterion_main, Criterion};

use agui::{manager::WidgetManager, widgets::primitives::Column};

fn widget_manager_ops(c: &mut Criterion) {
    c.bench_function("add to widget manager", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Column::default().into());
                }

                (WidgetManager::new(), column)
            },
            |(mut manager, widget)| {
                manager.set_root(widget);

                manager.update();
            },
        )
    });

    c.bench_function("remove from widget manager", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Column::default().into());
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
}

criterion_group!(benches, widget_manager_ops,);
criterion_main!(benches);
