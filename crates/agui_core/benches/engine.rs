use std::{cell::RefCell, rc::Rc};

use agui_core::{
    element::mock::{render::MockRenderWidget, DummyRenderObject, DummyWidget},
    engine::Engine,
    widget::{IntoWidget, Widget},
};
use criterion::{criterion_group, criterion_main, Criterion};

fn engine_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("engine (single)");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(500).bench_function("additions", |b| {
        b.iter_with_setup(
            || Engine::builder().with_root(DummyWidget).build(),
            |mut engine| engine.update(),
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let children = Rc::new(RefCell::new(vec![DummyWidget.into_widget()]));

                let root_widget = MockRenderWidget::default();
                {
                    root_widget
                        .mock
                        .borrow_mut()
                        .expect_children()
                        .returning_st({
                            let children = Rc::clone(&children);

                            move || children.borrow().clone()
                        });

                    root_widget
                        .mock
                        .borrow_mut()
                        .expect_create_render_object()
                        .returning(|_| DummyRenderObject.into());
                }

                let mut engine = Engine::builder().with_root(root_widget).build();

                engine.update();

                children.borrow_mut().clear();

                engine.mark_needs_build(engine.root());

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
                let children = {
                    let mut children = Vec::new();

                    for _ in 0..1000 {
                        children.push(DummyWidget.into_widget());
                    }

                    children
                };

                let root_widget = MockRenderWidget::default();
                {
                    root_widget
                        .mock
                        .borrow_mut()
                        .expect_children()
                        .returning_st(move || children.clone());

                    root_widget
                        .mock
                        .borrow_mut()
                        .expect_create_render_object()
                        .returning(|_| DummyRenderObject.into());
                }

                Engine::builder().with_root(root_widget).build()
            },
            |mut engine| engine.update(),
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let children = Rc::new(RefCell::new({
                    let mut children: Vec<Widget> = Vec::new();

                    for _ in 0..1000 {
                        children.push(DummyWidget.into_widget());
                    }

                    children
                }));

                let root_widget = MockRenderWidget::default();
                {
                    root_widget
                        .mock
                        .borrow_mut()
                        .expect_children()
                        .returning_st({
                            let children = Rc::clone(&children);

                            move || children.borrow().clone()
                        });

                    root_widget
                        .mock
                        .borrow_mut()
                        .expect_create_render_object()
                        .returning(|_| DummyRenderObject.into());
                }

                let mut engine = Engine::builder().with_root(root_widget).build();

                engine.update();

                children.borrow_mut().clear();

                engine.mark_needs_build(engine.root());

                engine
            },
            |mut engine| engine.update(),
        )
    });

    group.finish();
}

criterion_group!(benches, engine_ops);
criterion_main!(benches);
