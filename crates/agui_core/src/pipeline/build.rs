use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    context::{MessageCtx, UpdateCtx},
    element::{
        BoundaryId, BuildBoundaryElement, BuildState, RoutingPath, deliver_message,
        flush_boundaries, mark_rebuild,
    },
    provide::ProvideScope,
    render_object::RenderObject,
    scheduling::TaskScheduler,
    widget::{AnyWidget, Widget},
};

/// Owns the element tree built from one root widget, and the boundaries waiting to rebuild.
///
/// The root widget is registered as the outermost build boundary, so every element lives under a
/// boundary and a rebuild dispatches straight into the nearest one without a walk from the root.
pub struct BuildOwner<V: Widget>
where
    V::Render: RenderObject,
{
    provide: ProvideScope,

    state: Rc<RefCell<BuildState>>,

    root: BuildBoundaryElement<V::Render>,
}

impl<V> BuildOwner<V>
where
    V: Widget + 'static,
    V::Render: RenderObject,
{
    pub fn mount(widget: V, scheduler: &mut dyn TaskScheduler) -> Self {
        let provide = ProvideScope::new();
        let (state, root_scope) = BuildState::new();

        let recipe: Rc<dyn AnyWidget<Render = V::Render>> = Rc::new(widget);

        let root = {
            let mut path = Vec::new();
            let mut ctx = UpdateCtx::new(scheduler, &mut path, &provide, &root_scope);

            BuildBoundaryElement::create_rc(recipe, &mut ctx)
        };

        Self {
            provide,
            state,
            root,
        }
    }

    pub fn create_render_object(&mut self) -> V::Render {
        self.root.create_render_object()
    }

    pub fn update_render_object(&mut self, render_object: &mut V::Render) {
        self.root.update_render_object(render_object);
    }

    /// The id of the root boundary, for addressing a root-relative path.
    pub fn root_id(&self) -> BoundaryId {
        self.root.id()
    }

    /// Delivers `message` to the element at `path`. If that element asks to rebuild, its boundary is
    /// marked for the next [`flush`](Self::flush).
    pub fn dispatch_message(&mut self, path: &RoutingPath, message: Box<dyn Any>) {
        let mut ctx = MessageCtx::new(message);

        deliver_message(&self.state, path, &mut ctx);
    }

    /// Marks the element at `path` to rebuild on the next [`flush`](Self::flush).
    pub fn request_rebuild(&mut self, path: &RoutingPath) {
        mark_rebuild(&self.state, path);
    }

    /// Whether any boundary is waiting to rebuild.
    pub fn is_dirty(&self) -> bool {
        self.state.borrow().is_dirty()
    }

    /// Rebuilds every boundary marked since the last flush. Returns whether anything rebuilt, so the
    /// caller can skip reconciling the render tree when nothing changed.
    pub fn flush(&mut self, scheduler: &mut dyn TaskScheduler) -> bool {
        flush_boundaries(&self.state, scheduler, &self.provide)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        prelude::{element::*, render_object::*},
        test_fixtures::*,
        test_harness::*,
    };

    use super::BuildOwner;

    /// A shared counter, so a widget's closures can be `'static` and still be observed by the test.
    fn counter() -> Rc<Cell<usize>> {
        Rc::new(Cell::new(0))
    }

    /// A rebuild handler that bumps `c`.
    fn bump(c: &Rc<Cell<usize>>) -> impl Fn(&mut UpdateCtx) + 'static {
        let c = Rc::clone(c);
        move |_| c.set(c.get() + 1)
    }

    /// Records the addressed element's path on mount, so a test can dispatch to it without naming it.
    type Captured = Rc<RefCell<Option<RoutingPath>>>;

    fn capturing(slot: &Captured) -> impl Fn(&mut UpdateCtx) + 'static {
        let slot = Rc::clone(slot);
        move |ctx| *slot.borrow_mut() = Some(ctx.routing_path())
    }

    #[test]
    fn message_dirties_target_and_rebuild_only_reaches_that_target() {
        //   MultiChild
        //   ├─ [0] Transparent -> Leaf (no rebuild)
        //   ├─ [1] Transparent -> Leaf (requests rebuild)
        //   └─ [2] Transparent -> Leaf (no rebuild)
        let r0 = counter();
        let r1 = counter();
        let r2 = counter();

        let widget = MultiChild {
            children: vec![
                Transparent {
                    child: Leaf::new().on_rebuild(bump(&r0)),
                },
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(bump(&r1)),
                },
                Transparent {
                    child: Leaf::new().on_rebuild(bump(&r2)),
                },
            ],
        };

        let mut tasks = TestTaskRunner::new();
        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        let target = RoutingPath::new(owner.root_id(), vec![RoutingId::new(1)]);
        owner.dispatch_message(&target, Box::new(42_u32));
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
        let r0 = counter();
        let r1 = counter();
        let r2 = counter();

        let widget = MultiChild {
            children: vec![
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(bump(&r0)),
                },
                Transparent {
                    child: Leaf::new().on_rebuild(bump(&r1)),
                },
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(bump(&r2)),
                },
            ],
        };

        let mut tasks = TestTaskRunner::new();
        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        let root = owner.root_id();
        owner.dispatch_message(
            &RoutingPath::new(root, vec![RoutingId::new(0)]),
            Box::new(1_u32),
        );
        owner.dispatch_message(
            &RoutingPath::new(root, vec![RoutingId::new(2)]),
            Box::new(2_u32),
        );

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 1);
        assert_eq!(r1.get(), 0);
        assert_eq!(r2.get(), 1);
    }

    #[test]
    fn rebuild_under_rc_boundary_is_captured_and_reaches_only_that_leaf() {
        // MultiChild
        // ├─ [0] Rc<Leaf>  (boundary; requests rebuild)
        // └─ [1] Rc<Leaf>  (boundary; quiet)
        let r0 = counter();
        let r1 = counter();
        let target = Captured::default();

        let widget = MultiChild {
            children: vec![
                Rc::new(
                    Leaf::new()
                        .on_mount(capturing(&target))
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(bump(&r0)),
                ),
                Rc::new(Leaf::new().on_rebuild(bump(&r1))),
            ],
        };

        let mut tasks = TestTaskRunner::new();
        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        // The leaf sits under its own Rc boundary, addressed directly rather than from the root.
        let path = target.borrow().clone().expect("leaf was mounted");
        assert_ne!(
            path.boundary(),
            owner.root_id(),
            "addressed by its own boundary"
        );

        owner.dispatch_message(&path, Box::new(1_u32));
        assert!(owner.is_dirty(), "the boundary took the requested rebuild");

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 1, "the boundary's leaf rebuilt");
        assert_eq!(r1.get(), 0, "the sibling boundary was untouched");
        assert!(!owner.is_dirty(), "the flush cleared the boundary");
    }

    #[test]
    fn rebuild_routes_to_the_innermost_rc_boundary() {
        // Rc<Transparent<Rc<Leaf>>>: an outer boundary wrapping a transparent wrapper around an inner
        // boundary. The leaf is addressed by the inner boundary.
        let rebuilds = counter();
        let target = Captured::default();

        let widget = Rc::new(Transparent {
            child: Rc::new(
                Leaf::new()
                    .on_mount(capturing(&target))
                    .on_message(MessageCtx::request_rebuild)
                    .on_rebuild(bump(&rebuilds)),
            ),
        });

        let mut tasks = TestTaskRunner::new();
        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        let path = target.borrow().clone().expect("leaf was mounted");
        owner.dispatch_message(&path, Box::new(1_u32));
        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(rebuilds.get(), 1);
    }

    #[test]
    fn flush_with_empty_set_is_noop() {
        let r0 = counter();

        let widget = Leaf::new().on_rebuild(bump(&r0));

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

        let received = Rc::new(Cell::new(None::<u32>));
        let handle = Rc::new(RefCell::new(None::<TaskHandle>));

        let widget = Leaf::new()
            .on_mount({
                let handle = Rc::clone(&handle);
                move |ctx| {
                    *handle.borrow_mut() = Some(
                        ctx.spawn(|task| async move {
                            task.send(7_u32);
                        })
                        .expect("scheduler available during build"),
                    );
                }
            })
            .on_message({
                let received = Rc::clone(&received);
                move |ctx| received.set(Some(ctx.consume::<u32>()))
            });

        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        tasks.run_to_completion();

        let messages: Vec<_> = tasks.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        for (path, message) in messages {
            owner.dispatch_message(&path, message);
        }

        assert_eq!(received.get(), Some(7));
    }

    struct Parent {
        children: Vec<Leaf<'static>>,
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

    impl Widget for Parent {
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

        let a_messages = counter();
        let a_rebuilds = counter();
        let b_rebuilds = counter();
        let a_handle = Rc::new(RefCell::new(None::<TaskHandle>));

        let widget = Parent {
            children: vec![
                Leaf::new()
                    .on_mount({
                        let a_handle = Rc::clone(&a_handle);
                        move |ctx| {
                            *a_handle.borrow_mut() = Some(
                                ctx.spawn(|task| async move {
                                    task.send(42_u32);
                                })
                                .expect("scheduler available during build"),
                            );
                        }
                    })
                    .on_message({
                        let a_messages = Rc::clone(&a_messages);
                        move |ctx| {
                            a_messages.set(a_messages.get() + 1);
                            let _ = ctx.consume::<u32>();
                            ctx.request_rebuild();
                        }
                    })
                    .on_rebuild(bump(&a_rebuilds)),
                Leaf::new().on_rebuild(bump(&b_rebuilds)),
            ],
        };

        let mut owner = BuildOwner::mount(widget, &mut tasks.scheduler());

        tasks.run_to_completion();

        let messages: Vec<_> = tasks.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        for (path, message) in messages {
            owner.dispatch_message(&path, message);
        }

        assert_eq!(a_messages.get(), 1);
        assert!(owner.is_dirty(), "only the messaged child was dirtied");

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(a_rebuilds.get(), 1, "child 0 rebuilt");
        assert_eq!(b_rebuilds.get(), 0, "sibling subtree was not rebuilt");
    }
}
