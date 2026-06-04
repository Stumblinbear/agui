use std::any::Any;

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    provide::ProvideScope,
    routing_id::RoutingPath,
    task::scheduler::TaskScheduler,
    widget::Widget,
};

pub fn dispatch_messages<V: Widget>(
    root_element: &mut V::Element,
    root_widget: &V,
    messages: impl Iterator<Item = (RoutingPath, Box<dyn Any>)>,
    mut on_dirty: impl FnMut(RoutingPath),
) {
    for (path, message) in messages {
        let mut ctx = MessageCtx::new(message);

        root_widget.dispatch(root_element, path.as_slice(), Dispatch::Message(&mut ctx));

        if ctx.rebuild_requested() {
            on_dirty(path);
        }
    }
}

pub fn rebuild_dirty<V: Widget>(
    scheduler: &mut dyn TaskScheduler,
    provide_scope: &ProvideScope,
    root_element: &mut V::Element,
    root_widget: &V,
    dirty: impl ExactSizeIterator<Item = RoutingPath>,
) {
    for path in dirty {
        let slice = path.as_slice();

        let mut routing_path = path.to_vec();

        root_widget.dispatch(
            root_element,
            slice,
            Dispatch::Rebuild(&mut UpdateCtx::new(
                &mut *scheduler,
                &mut routing_path,
                provide_scope.clone(),
            )),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        constraints::Constraints,
        context::{Dispatch, MessageCtx, UpdateCtx},
        driver::{dispatch_messages, rebuild_dirty},
        element::MultiChildElement,
        hit_test::{HitTest, HitTestResult},
        offset::Offset,
        paint::PaintCtx,
        provide::ProvideScope,
        render_object::{MountCtx, RenderLeaf, RenderNode, RenderObject, box_layout::RenderBox},
        routing_id::{RoutingId, RoutingPath},
        size::Size,
        task::TaskHandle,
        test_fixtures::{Leaf, MultiChild, Transparent},
        test_harness::TestTaskRunner,
        text_baseline::TextBaseline,
        widget::Widget,
    };

    #[test]
    fn message_dirties_target_and_rebuild_only_reaches_that_target() {
        //   MultiChild
        //   ├─ [0] Transparent -> Leaf (no rebuild)
        //   ├─ [1] Transparent -> Leaf (requests rebuild)
        //   └─ [2] Transparent -> Leaf (no rebuild)
        let r0 = Cell::new(0_usize);
        let r1 = Cell::new(0_usize);
        let r2 = Cell::new(0_usize);

        let widget = MultiChild {
            children: vec![
                Transparent {
                    child: Leaf::new().on_rebuild(|_| r0.set(r0.get() + 1)),
                },
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(|_| r1.set(r1.get() + 1)),
                },
                Transparent {
                    child: Leaf::new().on_rebuild(|_| r2.set(r2.get() + 1)),
                },
            ],
        };

        let mut task_runner = TestTaskRunner::new();
        let provide_scope = ProvideScope::new();

        let mut routing_path = Vec::new();
        let mut root = widget.create_element(&mut UpdateCtx::new(
            &mut task_runner.scheduler(),
            &mut routing_path,
            provide_scope.clone(),
        ));

        let target_path: RoutingPath = vec![RoutingId::new(1)].into();
        let messages = vec![(target_path, Box::new(42_u32) as Box<dyn std::any::Any>)];

        let mut dirty: Vec<RoutingPath> = Vec::new();
        dispatch_messages(&mut root, &widget, messages.into_iter(), |path| {
            dirty.push(path);
        });

        assert_eq!(dirty.len(), 1, "exactly one element should be dirtied");
        assert_eq!(dirty[0].as_slice(), &[RoutingId::new(1)]);

        rebuild_dirty(
            &mut task_runner.scheduler(),
            &provide_scope,
            &mut root,
            &widget,
            dirty.into_iter(),
        );

        assert_eq!(r0.get(), 0);
        assert_eq!(r1.get(), 1);
        assert_eq!(r2.get(), 0);
    }

    #[test]
    fn rebuild_dirty_with_multiple_paths() {
        let r0 = Cell::new(0_usize);
        let r1 = Cell::new(0_usize);
        let r2 = Cell::new(0_usize);

        let widget = MultiChild {
            children: vec![
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(|_| r0.set(r0.get() + 1)),
                },
                Transparent {
                    child: Leaf::new().on_rebuild(|_| r1.set(r1.get() + 1)),
                },
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(|_| r2.set(r2.get() + 1)),
                },
            ],
        };

        let mut task_runner = TestTaskRunner::new();
        let provide_scope = ProvideScope::new();

        let mut routing_path = Vec::new();
        let mut root = widget.create_element(&mut UpdateCtx::new(
            &mut task_runner.scheduler(),
            &mut routing_path,
            provide_scope.clone(),
        ));

        let path_0: RoutingPath = vec![RoutingId::new(0)].into();
        let path_2: RoutingPath = vec![RoutingId::new(2)].into();
        let messages = vec![
            (path_0, Box::new(1_u32) as Box<dyn std::any::Any>),
            (path_2, Box::new(2_u32) as Box<dyn std::any::Any>),
        ];

        let mut dirty: Vec<RoutingPath> = Vec::new();
        dispatch_messages(&mut root, &widget, messages.into_iter(), |path| {
            dirty.push(path);
        });

        assert_eq!(dirty.len(), 2);

        rebuild_dirty(
            &mut task_runner.scheduler(),
            &provide_scope,
            &mut root,
            &widget,
            dirty.into_iter(),
        );

        assert_eq!(r0.get(), 1);
        assert_eq!(r1.get(), 0);
        assert_eq!(r2.get(), 1);
    }

    #[allow(clippy::let_unit_value)]
    #[test]
    fn rebuild_dirty_with_empty_set_is_noop() {
        let r0 = Cell::new(0_usize);

        let widget = Leaf::new().on_rebuild(|_| r0.set(r0.get() + 1));

        let mut task_runner = TestTaskRunner::new();
        let provide_scope = ProvideScope::new();

        let mut routing_path = Vec::new();
        let mut root = widget.create_element(&mut UpdateCtx::new(
            &mut task_runner.scheduler(),
            &mut routing_path,
            provide_scope.clone(),
        ));

        let dirty: Vec<RoutingPath> = Vec::new();
        rebuild_dirty(
            &mut task_runner.scheduler(),
            &provide_scope,
            &mut root,
            &widget,
            dirty.into_iter(),
        );

        assert_eq!(r0.get(), 0);
    }

    #[allow(clippy::let_unit_value)]
    #[test]
    fn spawned_task_posts_message_back_to_its_element() {
        // A leaf spawns a task on mount and stashes its handle (as a real element would) so the
        // task outlives the build. The runner drives it to completion; the task posts a message
        // back to its own routing path, which dispatching then delivers to the same leaf.
        let mut task_runner = TestTaskRunner::new();
        let provide_scope = ProvideScope::new();

        let received = Cell::new(None::<u32>);
        let handle = RefCell::new(None::<TaskHandle>);

        let widget = Leaf::new()
            .on_mount(|ctx| {
                *handle.borrow_mut() = Some(
                    ctx.spawn(|task| async move {
                        task.send(7_u32);
                    })
                    .expect("scheduler available during build"),
                );
            })
            .on_message(|ctx| received.set(Some(ctx.consume::<u32>())));

        let mut routing_path = Vec::new();
        let mut root = widget.create_element(&mut UpdateCtx::new(
            &mut task_runner.scheduler(),
            &mut routing_path,
            provide_scope.clone(),
        ));

        task_runner.run_to_completion();

        let messages: Vec<_> = task_runner.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        dispatch_messages(&mut root, &widget, messages.into_iter(), |_| {});

        assert_eq!(received.get(), Some(7));
    }

    struct Parent<'a> {
        children: Vec<Leaf<'a>>,
    }

    struct RenderParent {
        children: Vec<RenderNode<RenderLeaf>>,
    }

    impl RenderObject for RenderParent {
        fn mount(&mut self, ctx: &mut MountCtx) {
            for child in &mut self.children {
                child.mount(ctx);
            }
        }

        fn unmount(&mut self, ctx: &mut MountCtx) {
            for child in &mut self.children {
                child.unmount(ctx);
            }
        }

        fn update_compositing_bits(&mut self) -> bool {
            let mut needs = false;

            for child in &mut self.children {
                needs |= child.update_compositing_bits();
            }

            needs
        }
    }

    impl RenderBox for RenderParent {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn measure(&self, constraints: Constraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, constraints: Constraints) -> Size {
            for child in &mut self.children {
                child.layout(constraints);
            }

            constraints.smallest()
        }

        fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, ctx: &mut PaintCtx) {
            for child in &mut self.children {
                child.paint(ctx);
            }
        }
    }

    impl Widget for Parent<'_> {
        type Element = MultiChildElement<()>;

        type Render = RenderParent;

        fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
            MultiChildElement::new(self.children.len(), |i| &self.children[i], ctx)
        }

        fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
            element.update(
                self.children.len(),
                |i| &self.children[i],
                |i| &old.children[i],
                ctx,
            );
        }

        fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
            element.dispatch(|i| &self.children[i], path, action);
        }

        fn create_render_object(&self, element: &Self::Element) -> Self::Render {
            RenderParent {
                children: self
                    .children
                    .iter()
                    .zip(&element.children)
                    .map(|(child, node)| RenderNode::new(child.create_render_object(&node.element)))
                    .collect(),
            }
        }

        fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
            for ((child, node), child_render) in self
                .children
                .iter()
                .zip(&element.children)
                .zip(&mut render_object.children)
            {
                child.update_render_object(&node.element, &mut child_render.object);
            }
        }
    }

    #[test]
    fn task_message_rebuilds_only_its_subtree() {
        let mut task_runner = TestTaskRunner::new();
        let provide_scope = ProvideScope::new();

        let a_messages = Cell::new(0_usize);
        let a_rebuilds = Cell::new(0_usize);
        let b_rebuilds = Cell::new(0_usize);
        let a_handle = RefCell::new(None::<TaskHandle>);

        let widget = Parent {
            children: vec![
                Leaf::new()
                    .on_mount(|ctx| {
                        *a_handle.borrow_mut() = Some(
                            ctx.spawn(|task| async move {
                                task.send(42_u32);
                            })
                            .expect("scheduler available during build"),
                        );
                    })
                    .on_message(|ctx| {
                        a_messages.set(a_messages.get() + 1);
                        let _ = ctx.consume::<u32>();
                        ctx.request_rebuild();
                    })
                    .on_rebuild(|_| a_rebuilds.set(a_rebuilds.get() + 1)),
                Leaf::new().on_rebuild(|_| b_rebuilds.set(b_rebuilds.get() + 1)),
            ],
        };

        let mut routing_path = Vec::new();
        let mut root = widget.create_element(&mut UpdateCtx::new(
            &mut task_runner.scheduler(),
            &mut routing_path,
            provide_scope.clone(),
        ));

        task_runner.run_to_completion();

        let messages: Vec<_> = task_runner.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        let mut dirty: Vec<RoutingPath> = Vec::new();
        dispatch_messages(&mut root, &widget, messages.into_iter(), |path| {
            dirty.push(path);
        });

        assert_eq!(a_messages.get(), 1);
        assert_eq!(dirty.len(), 1, "only child 0 was dirtied");
        assert_eq!(dirty[0].as_slice(), &[RoutingId::from_index(0)]);

        rebuild_dirty(
            &mut task_runner.scheduler(),
            &provide_scope,
            &mut root,
            &widget,
            dirty.into_iter(),
        );

        assert_eq!(a_rebuilds.get(), 1, "child 0 rebuilt");
        assert_eq!(b_rebuilds.get(), 0, "sibling subtree was not rebuilt");
    }
}
