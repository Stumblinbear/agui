use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    context::{MessageCtx, UpdateCtx},
    element::{
        BuildBoundaryElement, BuildBoundaryId, BuildState, Element, RoutingPath, deliver_message,
        flush_boundaries, mark_rebuild,
    },
    prelude::element::AnyRenderObject,
    provide::ProvideScope,
    render_object::RenderObject,
    scheduling::TaskScheduler,
    widget::Widget,
};

/// Owns the element tree built from one root widget, and the boundaries waiting to rebuild.
///
/// The root widget is registered as the outermost build boundary, so every element lives under a
/// boundary and a rebuild dispatches straight into the nearest one without a walk from the root.
pub struct BuildOwner {
    provide: ProvideScope,

    state: Rc<RefCell<BuildState>>,

    root: BuildBoundaryElement,
}

impl BuildOwner {
    /// Mounts `widget` as the root boundary, returning the owner and a shared cell holding the root's
    /// render object.
    pub fn mount<V>(widget: V, scheduler: &mut dyn TaskScheduler) -> (Self, Rc<RefCell<V::Render>>)
    where
        V: Widget + 'static,
        V::Element: 'static,
        <V::Element as Element>::Render: AnyRenderObject + Sized,
        V::Render: RenderObject + 'static,
    {
        let provide = ProvideScope::new();
        let (state, root_scope) = BuildState::new();

        let (root, render) = {
            let mut path = Vec::new();
            let mut ctx = UpdateCtx::new(scheduler, &mut path, &provide, &root_scope);

            BuildBoundaryElement::create(widget, &mut ctx)
        };

        (
            Self {
                provide,
                state,
                root,
            },
            render,
        )
    }

    /// The id of the root boundary, for addressing a root-relative path.
    pub fn root_id(&self) -> BuildBoundaryId {
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

    use crate::{prelude::element::*, test_fixtures::*, test_harness::*};

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
        let (mut owner, _) = BuildOwner::mount(widget, &mut tasks.scheduler());

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
        let (mut owner, _) = BuildOwner::mount(widget, &mut tasks.scheduler());

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
    fn flush_with_empty_set_is_noop() {
        let r0 = counter();

        let widget = Leaf::new().on_rebuild(bump(&r0));

        let mut tasks = TestTaskRunner::new();
        let (mut owner, _) = BuildOwner::mount(widget, &mut tasks.scheduler());

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

        let (mut owner, _) = BuildOwner::mount(widget, &mut tasks.scheduler());

        tasks.run_to_completion();

        let messages: Vec<_> = tasks.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        for (path, message) in messages {
            owner.dispatch_message(&path, message);
        }

        assert_eq!(received.get(), Some(7));
    }

    #[test]
    fn task_message_rebuilds_only_the_messaged_child() {
        let mut tasks = TestTaskRunner::new();

        let a_messages = counter();
        let a_rebuilds = counter();
        let b_rebuilds = counter();
        let a_handle = Rc::new(RefCell::new(None::<TaskHandle>));

        let widget = MultiChild {
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

        let (mut owner, _) = BuildOwner::mount(widget, &mut tasks.scheduler());

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
        assert_eq!(b_rebuilds.get(), 0, "the sibling was not rebuilt");
    }
}
