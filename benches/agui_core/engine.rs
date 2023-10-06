use std::cell::RefCell;

use agui_core::{
    unit::{Constraints, IntrinsicDimension, Size},
    widget::{IntoWidget, IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;
use agui_primitives::sized_box::SizedBox;
use criterion::{criterion_group, criterion_main, Criterion};

use agui::{engine::Engine, widgets::primitives::flex::Column};

fn engine_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine (single)");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(500).bench_function("additions", |b| {
        b.iter_with_setup(
            || {
                Engine::builder()
                    .with_root(Column::builder().build())
                    .build()
            },
            |mut engine| {
                engine.update();
            },
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let mut engine = Engine::builder()
                    .with_root(TestRootWidget::builder().build())
                    .build();

                TEST_HOOK.with(|children| {
                    *children.borrow_mut() = Vec::from([SizedBox::builder().build().into_widget()]);
                });

                engine.update();

                TEST_HOOK.with(|children| {
                    *children.borrow_mut() = Vec::new();
                });

                engine
            },
            |mut engine| engine.update(),
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

                Engine::builder().with_root(column).build()
            },
            |mut engine| {
                engine.update();
            },
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let mut engine = Engine::builder()
                    .with_root(TestRootWidget::builder().build())
                    .build();

                TEST_HOOK.with(|children| {
                    let mut arr = Vec::new();

                    for _ in 0..1000 {
                        arr.push(SizedBox::builder().build().into_widget());
                    }

                    *children.borrow_mut() = arr;
                });

                engine.update();

                TEST_HOOK.with(|children| {
                    *children.borrow_mut() = Vec::new();
                });

                engine
            },
            |mut engine| {
                engine.update();
            },
        )
    });

    group.finish();
}

thread_local! {
    static TEST_HOOK: RefCell<Vec<Widget>> = RefCell::default();
}

#[derive(Default, LayoutWidget)]
struct TestRootWidget;

impl WidgetLayout for TestRootWidget {
    fn get_children(&self) -> Vec<Widget> {
        Vec::from_iter(TEST_HOOK.with(|result| result.borrow().clone()))
    }

    fn intrinsic_size(&self, _: &mut IntrinsicSizeContext, _: IntrinsicDimension, _: f32) -> f32 {
        0.0
    }

    fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
        Size::ZERO
    }
}

impl TestRootWidget {
    fn set_children(children: Vec<Widget>) {
        TEST_HOOK.with(|result| {
            *result.borrow_mut() = children;
        });
    }
}

criterion_group!(benches, engine_ops);
criterion_main!(benches);
