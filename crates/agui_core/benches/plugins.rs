use std::{cell::RefCell, rc::Rc};

use agui_core::{
    element::mock::{render::MockRenderWidget, DummyRenderObject, DummyWidget},
    engine::Engine,
    plugin::{
        context::{
            PluginAfterUpdateContext, PluginBeforeUpdateContext, PluginCreateRenderObjectContext,
            PluginElementBuildContext, PluginElementMountContext, PluginElementRemountContext,
            PluginElementUnmountContext, PluginInitContext, UpdatePluginRenderObjectContext,
        },
        Plugin,
    },
    widget::IntoWidget,
};
use criterion::{criterion_group, criterion_main, Criterion};

fn plugin_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("plugins");

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

                Engine::builder()
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .with_root(root_widget)
                    .build()
            },
            |mut engine| engine.update(),
        )
    });

    group.sample_size(500).bench_function("removals", |b| {
        b.iter_with_setup(
            || {
                let children = Rc::new(RefCell::new({
                    let mut children = Vec::new();

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

                let mut engine = Engine::builder()
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .add_plugin(DummyPlugin::default())
                    .with_root(root_widget)
                    .build();

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

#[derive(Default)]
struct DummyPlugin {
    i: usize,
}

impl Plugin for DummyPlugin {
    fn on_init(&mut self, _: &mut PluginInitContext) {
        self.i += 1;
    }

    fn on_after_update(&mut self, _: &mut PluginAfterUpdateContext) {
        self.i += 1;
    }

    fn on_element_mount(&mut self, _: &mut PluginElementMountContext) {
        self.i += 1;
    }

    fn on_element_remount(&mut self, _: &mut PluginElementRemountContext) {
        self.i += 1;
    }

    fn on_element_unmount(&mut self, _: &mut PluginElementUnmountContext) {
        self.i += 1;
    }

    fn on_element_build(&mut self, _: &mut PluginElementBuildContext) {
        self.i += 1;
    }

    fn on_create_render_object(&mut self, _: &mut PluginCreateRenderObjectContext) {
        self.i += 1;
    }

    fn on_update_render_object(&mut self, _: &mut UpdatePluginRenderObjectContext) {
        self.i += 1;
    }

    fn on_before_update(&mut self, _: &mut PluginBeforeUpdateContext) {
        self.i += 1;
    }
}

criterion_group!(benches, plugin_ops);
criterion_main!(benches);
