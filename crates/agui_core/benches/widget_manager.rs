use std::{cell::RefCell, rc::Rc};

use agui_core::{
    element::mock::{render::MockRenderWidget, DummyRenderObject, DummyWidget},
    engine::widgets::WidgetManager,
    widget::{IntoWidget, Widget},
};
use criterion::{criterion_group, criterion_main, Criterion};

fn widget_manager_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("widget manager (single)");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(10000).bench_function("additions", |b| {
        b.iter_with_setup(WidgetManager::default, |manager| {
            manager.with_root(DummyWidget.into_widget())
        })
    });

    group.sample_size(10000).bench_function("rebuilds", |b| {
        b.iter_with_setup(
            || {
                let mut manager = WidgetManager::default_with_root(DummyWidget.into_widget());

                manager.mark_needs_build(manager.root().expect("no root element"));

                manager
            },
            |mut manager| manager.update(),
        )
    });

    group.sample_size(10000).bench_function("removals", |b| {
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

                let mut manager = WidgetManager::default_with_root(root_widget.into_widget());

                children.borrow_mut().clear();

                manager.mark_needs_build(manager.root().expect("no root element"));

                manager
            },
            |mut manager| manager.update(),
        )
    });

    group.finish();

    let mut group = c.benchmark_group("widget manager (large)");

    group.throughput(criterion::Throughput::Elements(1000));

    group.sample_size(1000).bench_function("additions", |b| {
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

                (WidgetManager::default(), root_widget.into_widget())
            },
            |(manager, root_widget)| manager.with_root(root_widget),
        )
    });

    group.sample_size(1000).bench_function("rebuilds", |b| {
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

                let mut manager = WidgetManager::default_with_root(root_widget.into_widget());

                manager.mark_needs_build(manager.root().expect("no root element"));

                manager
            },
            |mut manager| manager.update(),
        )
    });

    group.sample_size(1000).bench_function("removals", |b| {
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

                let mut manager = WidgetManager::default_with_root(root_widget.into_widget());

                children.borrow_mut().clear();

                manager.mark_needs_build(manager.root().expect("no root element"));

                manager
            },
            |mut manager| manager.update(),
        )
    });

    group.finish();
}

criterion_group!(benches, widget_manager_ops);
criterion_main!(benches);
