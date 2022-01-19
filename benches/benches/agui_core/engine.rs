use criterion::{criterion_group, criterion_main, Criterion};

use agui::{engine::Engine, widget::WidgetRef, widgets::primitives::Column};

fn engine_ops(c: &mut Criterion) {
    c.bench_function("add to engine", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Column::default().into());
                }

                (Engine::default(), WidgetRef::new(column))
            },
            |(mut engine, widget)| {
                engine.set_root(widget);

                engine.update();
            },
        )
    });

    c.bench_function("remove from engine", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Column::default().into());
                }

                let mut engine = Engine::default();

                engine.set_root(WidgetRef::new(column));

                engine.update();

                engine
            },
            |mut engine| {
                engine.set_root(WidgetRef::None);

                engine.update();
            },
        )
    });
}

criterion_group!(benches, engine_ops,);
criterion_main!(benches);
