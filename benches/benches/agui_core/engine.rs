use criterion::{criterion_group, criterion_main, Criterion};

use agui::{
    engine::{render::Renderer, Engine},
    widget::WidgetRef,
    widgets::primitives::Column,
};

struct BenchRenderer {}

struct BenchPicture {}

impl Renderer<BenchPicture> for BenchRenderer {
    fn draw(&self, _canvas: &agui::canvas::Canvas) -> BenchPicture {
        BenchPicture {}
    }

    fn render(&self, _picture: &BenchPicture) {}
}

fn engine_ops(c: &mut Criterion) {
    c.bench_function("add to engine", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Column::default().into());
                }

                (Engine::new(BenchRenderer {}), WidgetRef::new(column))
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

                let mut engine = Engine::new(BenchRenderer {});

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
