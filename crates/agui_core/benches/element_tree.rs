use std::{any::Any, cell::RefCell, rc::Rc, sync::Arc};

use agui_core::{
    callback::{strategies::CallbackStrategy, CallbackId},
    element::{
        mock::render::{MockRenderObject, MockRenderWidget},
        Element, ElementBuildContext, ElementComparison, ElementId, ElementMountContext,
        ElementUnmountContext,
    },
    engine::elements::{
        context::{ElementTreeContext, ElementTreeMountContext},
        scheduler::{CreateElementTask, ElementSchedulerStrategy},
        strategies::{InflateElementStrategy, UnmountElementStrategy},
        ElementTree,
    },
    task::{error::TaskError, TaskHandle},
    widget::{IntoWidget, Widget},
};
use criterion::{criterion_group, criterion_main, Criterion};

pub struct NoopInflateStrategy {
    callbacks: Arc<dyn CallbackStrategy>,
}

impl Default for NoopInflateStrategy {
    fn default() -> Self {
        Self {
            callbacks: Arc::new(NoopCallbackStratgy::default()),
        }
    }
}

impl InflateElementStrategy for NoopInflateStrategy {
    type Definition = Widget;

    fn mount(&mut self, ctx: ElementTreeMountContext, definition: Self::Definition) -> Element {
        let mut element = definition.create_element();

        element.mount(&mut ElementMountContext {
            element_tree: ctx.tree,

            parent_element_id: ctx.parent_element_id,
            element_id: ctx.element_id,
        });

        element
    }

    fn try_update(
        &mut self,
        _: ElementId,
        element: &mut Element,
        definition: &Self::Definition,
    ) -> ElementComparison {
        element.update(definition)
    }

    fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Self::Definition> {
        element.build(&mut ElementBuildContext {
            scheduler: &mut ctx
                .scheduler
                .with_strategy(&mut NoopSchedulerStratgy::default()),
            callbacks: &self.callbacks,

            element_tree: ctx.tree,
            inheritance: ctx.inheritance,

            element_id: ctx.element_id,
        })
    }
}

#[derive(Default, Clone)]
struct NoopUnmountStrategy {}

impl UnmountElementStrategy for NoopUnmountStrategy {
    fn unmount(&mut self, _: ElementUnmountContext, _: Element) {}
}

#[derive(Default, Clone)]
pub struct NoopCallbackStratgy {}

impl CallbackStrategy for NoopCallbackStratgy {
    fn call_unchecked(&self, _: CallbackId, _: Box<dyn Any + Send>) {}
}

#[derive(Default, Clone)]
pub struct NoopSchedulerStratgy {}

impl ElementSchedulerStrategy for NoopSchedulerStratgy {
    fn spawn_task(&mut self, _: CreateElementTask) -> Result<TaskHandle<()>, TaskError> {
        Err(TaskError::no_scheduler())
    }
}

fn element_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("element tree (single)");

    group.throughput(criterion::Throughput::Elements(1));

    group.sample_size(10000).bench_function("additions", |b| {
        b.iter_with_setup(
            || {
                (
                    NoopInflateStrategy::default(),
                    ElementTree::default(),
                    MockRenderWidget::dummy(),
                )
            },
            |(mut inflate_strategy, mut tree, widget)| {
                tree.inflate(&mut inflate_strategy, widget)
                    .expect("failed to spawn and inflate");
            },
        )
    });

    group.sample_size(10000).bench_function("rebuilds", |b| {
        b.iter_with_setup(
            || {
                let mut tree = ElementTree::default();

                let element_id = tree
                    .inflate(
                        &mut NoopInflateStrategy::default(),
                        MockRenderWidget::dummy(),
                    )
                    .expect("failed to spawn and inflate");

                (NoopInflateStrategy::default(), tree, element_id)
            },
            |(mut inflate_strategy, mut tree, element_id)| {
                tree.rebuild(&mut inflate_strategy, element_id)
                    .expect("failed to build and realize")
            },
        )
    });

    group.sample_size(10000).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let children = Rc::new(RefCell::new(vec![MockRenderWidget::dummy()]));

                let root_widget = MockRenderWidget::default();
                {
                    root_widget.mock().expect_children().returning_st({
                        let children = Rc::clone(&children);

                        move || children.borrow().clone()
                    });

                    root_widget
                        .mock()
                        .expect_create_render_object()
                        .returning(|_| MockRenderObject::dummy());
                }
                let root_widget = root_widget.into_widget();

                let mut tree = ElementTree::default();

                tree.inflate(&mut NoopInflateStrategy::default(), root_widget)
                    .expect("failed to spawn and inflate");

                tree
            },
            |mut tree| {
                tree.clear(&mut NoopUnmountStrategy::default())
                    .expect("failed to clear tree")
            },
        )
    });

    group.finish();

    let mut group = c.benchmark_group("element tree (large)");

    group.throughput(criterion::Throughput::Elements(1000));

    group.sample_size(1000).bench_function("additions", |b| {
        b.iter_with_setup(
            || {
                let children = {
                    let mut children = Vec::new();

                    for _ in 0..1000 {
                        children.push(MockRenderWidget::dummy());
                    }

                    children
                };

                let root_widget = MockRenderWidget::default();
                {
                    root_widget
                        .mock()
                        .expect_children()
                        .returning_st(move || children.clone());
                }
                let root_widget = root_widget.into_widget();

                (
                    NoopInflateStrategy::default(),
                    ElementTree::default(),
                    root_widget,
                )
            },
            |(mut inflate_strategy, mut tree, widget)| {
                tree.inflate(&mut inflate_strategy, widget)
                    .expect("failed to spawn and inflate")
            },
        )
    });

    group.sample_size(1000).bench_function("rebuilds", |b| {
        b.iter_with_setup(
            || {
                let children = {
                    let mut children = Vec::new();

                    for _ in 0..1000 {
                        children.push(MockRenderWidget::dummy());
                    }

                    children
                };

                let root_widget = MockRenderWidget::default();
                {
                    root_widget
                        .mock()
                        .expect_children()
                        .returning_st(move || children.clone());
                }
                let root_widget = root_widget.into_widget();

                let mut tree = ElementTree::default();

                let element_id = tree
                    .inflate(&mut NoopInflateStrategy::default(), root_widget)
                    .expect("failed to spawn and inflate");

                (NoopInflateStrategy::default(), tree, element_id)
            },
            |(mut inflate_strategy, mut tree, element_id)| {
                tree.rebuild(&mut inflate_strategy, element_id)
                    .expect("failed to build and realize")
            },
        )
    });

    group.sample_size(1000).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let children = Rc::new(RefCell::new({
                    let mut children: Vec<Widget> = Vec::new();

                    for _ in 0..1000 {
                        children.push(MockRenderWidget::dummy());
                    }

                    children
                }));

                let root_widget = MockRenderWidget::default();
                {
                    root_widget.mock().expect_children().returning_st({
                        let children = Rc::clone(&children);

                        move || children.borrow().clone()
                    });

                    root_widget
                        .mock()
                        .expect_create_render_object()
                        .returning(|_| MockRenderObject::dummy());
                }
                let root_widget = root_widget.into_widget();

                let mut tree = ElementTree::default();

                tree.inflate(&mut NoopInflateStrategy::default(), root_widget)
                    .expect("failed to spawn and inflate");

                tree
            },
            |mut tree| {
                tree.clear(&mut NoopUnmountStrategy::default())
                    .expect("failed to clear tree")
            },
        )
    });

    group.finish();
}

criterion_group!(benches, element_tree);
criterion_main!(benches);
