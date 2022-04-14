use criterion::{criterion_group, criterion_main, Criterion};

use agui::{engine::Engine, widgets::primitives::Column};

fn engine_ops(c: &mut Criterion) {
    c.bench_function("add to engine", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Column::default().into());
                }

                (Engine::new(), column)
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
}

criterion_group!(benches, engine_ops,);
criterion_main!(benches);
