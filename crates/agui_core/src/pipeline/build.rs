use std::any::Any;

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    element::{RoutingPath, node::ElementNode},
    prelude::element::RoutingId,
    provide::ProvideScope,
    scheduling::TaskScheduler,
    widget::Widget,
};

/// Owns the element tree built from one root widget, and the elements waiting to rebuild.
pub struct BuildOwner<V: Widget> {
    widget: V,
    element: ElementNode<V::Element>,
    provide: ProvideScope,

    dirty: Vec<RoutingPath>,

    path: Vec<RoutingId>,
}

impl<V: Widget> BuildOwner<V> {
    pub fn mount(widget: V, scheduler: &mut dyn TaskScheduler) -> Self {
        let provide = ProvideScope::new();

        let element = {
            let mut path = Vec::new();
            let mut ctx = UpdateCtx::new(scheduler, &mut path, &provide);

            ElementNode::new(widget.create_element(&mut ctx))
        };

        Self {
            widget,

            element,
            provide,
            dirty: Vec::new(),

            path: Vec::new(),
        }
    }

    pub fn update_element(&mut self, widget: V, scheduler: &mut dyn TaskScheduler) {
        self.element.update(
            &widget,
            &self.widget,
            &mut UpdateCtx::new(scheduler, &mut self.path, &self.provide),
        );

        self.path.clear();

        self.widget = widget;
    }

    pub fn create_render_object(&mut self) -> V::Render {
        self.element.create_render_object(&self.widget)
    }

    pub fn update_render_object(&mut self, render_object: &mut V::Render) {
        self.element
            .update_render_object(&self.widget, render_object);
    }

    /// Delivers `message` to the element at `path` in the tree built from `widget`. If that element
    /// asks to rebuild, it is marked for the next [`flush`](Self::flush).
    pub fn dispatch_message(&mut self, path: RoutingPath, message: Box<dyn Any>) {
        let mut ctx = MessageCtx::new(message);

        self.element
            .dispatch(&self.widget, path.as_slice(), Dispatch::Message(&mut ctx));

        if ctx.rebuild_requested() {
            self.dirty.push(path);
        }
    }

    /// Marks the element at `path` to rebuild on the next [`flush`](Self::flush).
    pub fn request_rebuild(&mut self, path: RoutingPath) {
        self.dirty.push(path);
    }

    /// Whether any element is waiting to rebuild.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    /// Rebuilds every element marked since the last flush. Returns whether anything rebuilt,
    /// so the caller can skip reconciling the render tree when nothing changed.
    pub fn flush(&mut self, scheduler: &mut dyn TaskScheduler) -> bool {
        if self.dirty.is_empty() {
            return false;
        }

        for path in self.dirty.drain(..) {
            let slice = path.as_slice();

            let mut routing_path = path.to_vec();

            self.element.dispatch(
                &self.widget,
                slice,
                Dispatch::Rebuild(&mut UpdateCtx::new(
                    &mut *scheduler,
                    &mut routing_path,
                    &self.provide,
                )),
            );
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        prelude::{element::*, render_object::*},
        test_fixtures::*,
        test_harness::*,
    };

    use super::BuildOwner;

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

        let mut tasks = TestTaskRunner::new();
        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        owner.dispatch_message(vec![RoutingId::new(1)].into(), Box::new(42_u32));
        assert!(
            owner.is_dirty(),
            "the addressed element requested a rebuild"
        );

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 0);
        assert_eq!(r1.get(), 1);
        assert_eq!(r2.get(), 0);
    }

    #[test]
    fn rebuild_reaches_every_dirtied_target() {
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

        let mut tasks = TestTaskRunner::new();
        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        owner.dispatch_message(vec![RoutingId::new(0)].into(), Box::new(1_u32));
        owner.dispatch_message(vec![RoutingId::new(2)].into(), Box::new(2_u32));

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 1);
        assert_eq!(r1.get(), 0);
        assert_eq!(r2.get(), 1);
    }

    #[test]
    fn flush_with_empty_set_is_noop() {
        let r0 = Cell::new(0_usize);

        let widget = Leaf::new().on_rebuild(|_| r0.set(r0.get() + 1));

        let mut tasks = TestTaskRunner::new();
        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        assert!(!owner.is_dirty());
        assert!(!owner.flush(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 0);
    }

    #[test]
    fn spawned_task_posts_message_back_to_its_element() {
        // A leaf spawns a task on mount and stashes its handle (as a real element would) so the
        // task outlives the build. The runner drives it to completion; the task posts a message
        // back to its own routing path, which dispatching then delivers to the same leaf.
        let mut tasks = TestTaskRunner::new();

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

        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        tasks.run_to_completion();

        let messages: Vec<_> = tasks.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        for (path, message) in messages {
            owner.dispatch_message(path, message);
        }

        assert_eq!(received.get(), Some(7));
    }

    struct Parent<'a> {
        children: Vec<Leaf<'a>>,
    }

    struct RenderParent {
        children: Vec<RenderNode<()>>,
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

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            for child in &mut self.children {
                child.layout(ctx, constraints);
            }

            constraints.smallest()
        }

        fn measure_baseline(
            &self,
            _: BoxConstraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            for child in &mut self.children {
                child.paint(ctx, offset);
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
            #[allow(clippy::unit_arg)]
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
        let mut tasks = TestTaskRunner::new();

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

        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        tasks.run_to_completion();

        let messages: Vec<_> = tasks.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        for (path, message) in messages {
            owner.dispatch_message(path, message);
        }

        assert_eq!(a_messages.get(), 1);
        assert!(owner.is_dirty(), "only the messaged child was dirtied");

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(a_rebuilds.get(), 1, "child 0 rebuilt");
        assert_eq!(b_rebuilds.get(), 0, "sibling subtree was not rebuilt");
    }
}
