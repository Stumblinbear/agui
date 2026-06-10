use std::any::TypeId;

use crate::{context::UpdateCtx, element::Element, key::AnyKeyable};

mod any_widget;

pub use any_widget::*;

/// The immutable description of a piece of the tree. A `Widget` is consumed to build its persistent
/// [`Element`], which holds state and children, together with its render object; it is consumed
/// again on each reconcile to sync both.
///
/// The type of its render object is [`Render`](Self::Render).
pub trait Widget {
    type Element: Element<Render = Self::Render>;

    type Render;

    /// Builds this widget's persistent [`Element`] and its render object, consuming the description.
    /// Called once, when the widget first enters the tree.
    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render);

    /// Reconciles `element` and `render_object` against this new description, consuming it. Called
    /// when the parent supplies a new widget of the same type.
    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    );

    /// The type identity used, together with [`key`](Self::key), to decide whether a new widget
    /// reconciles an existing element in place rather than replacing it. Type-erased widgets report
    /// their concrete inner widget's identity.
    fn widget_type_id(&self) -> TypeId
    where
        Self: Sized + 'static,
    {
        TypeId::of::<Self>()
    }

    /// This is an implementation detail of element keys and should not be overriden by any user code.
    fn key(&self) -> Option<&dyn AnyKeyable> {
        None
    }
}

impl Widget for () {
    type Element = ();

    type Render = ();

    fn create(self, _: &mut UpdateCtx) -> ((), ()) {
        ((), ())
    }

    fn update(self, (): &mut (), (): &mut (), _: &mut UpdateCtx) {}
}

#[cfg(test)]
mod tests {
    use std::{any::Any, cell::Cell, rc::Rc};

    use crate::{
        context::{Dispatch, MessageCtx},
        element::{Element, RoutingId},
        test_fixtures::{Leaf, MultiChild, Transparent},
        test_harness::with_ctx,
        widget::Widget,
    };

    fn counter() -> Rc<Cell<usize>> {
        Rc::new(Cell::new(0))
    }

    /// A leaf that bumps `count` on each message it receives.
    fn counting_leaf(count: &Rc<Cell<usize>>) -> Leaf {
        let count = Rc::clone(count);
        Leaf::new().on_message(move |_| count.set(count.get() + 1))
    }

    /// A leaf that bumps `count` and records the message's `u32` payload.
    fn recording_leaf(count: &Rc<Cell<usize>>, payload: &Rc<Cell<Option<u32>>>) -> Leaf {
        let count = Rc::clone(count);
        let payload = Rc::clone(payload);
        Leaf::new().on_message(move |ctx| {
            count.set(count.get() + 1);
            payload.set(Some(ctx.consume::<u32>()));
        })
    }

    /// A leaf that bumps `count` on each rebuild.
    fn rebuilding_leaf(count: &Rc<Cell<usize>>) -> Leaf {
        let count = Rc::clone(count);
        Leaf::new().on_rebuild(move |_| count.set(count.get() + 1))
    }

    /// A leaf that bumps `count` on each update.
    fn updating_leaf(count: &Rc<Cell<usize>>) -> Leaf {
        let count = Rc::clone(count);
        Leaf::new().on_update(move |_| count.set(count.get() + 1))
    }

    fn mount<W: Widget>(widget: W) -> (W::Element, W::Render) {
        with_ctx(|ctx| widget.create(ctx))
    }

    fn message<E: Element>(
        element: &mut E,
        render: &mut E::Render,
        path: &[RoutingId],
        payload: Box<dyn Any>,
    ) -> MessageCtx {
        let mut ctx = MessageCtx::new(payload);
        element.dispatch(render, path, Dispatch::Message(&mut ctx));
        ctx
    }

    fn rebuild<E: Element>(element: &mut E, render: &mut E::Render, path: &[RoutingId]) {
        with_ctx(|ctx| element.dispatch(render, path, Dispatch::Rebuild(ctx)));
    }

    #[test]
    fn message_at_empty_path_invokes_leaf() {
        let count = counter();
        let payload = Rc::new(Cell::new(None));
        let (mut element, mut render) = mount(recording_leaf(&count, &payload));

        message(&mut element, &mut render, &[], Box::new(42_u32));

        assert_eq!(count.get(), 1);
        assert_eq!(payload.get(), Some(42));
    }

    #[test]
    fn rebuild_at_empty_path_invokes_leaf() {
        let count = counter();
        let (mut element, mut render) = mount(rebuilding_leaf(&count));

        rebuild(&mut element, &mut render, &[]);

        assert_eq!(count.get(), 1);
    }

    #[test]
    fn message_without_request_rebuild_leaves_flag_unset() {
        let (mut element, mut render) = mount(Leaf::new());

        let ctx = message(&mut element, &mut render, &[], Box::new(1_u32));

        assert!(
            !ctx.rebuild_requested(),
            "leaf did not call request_rebuild"
        );
    }

    #[test]
    fn message_with_request_rebuild_sets_flag() {
        let (mut element, mut render) = mount(Leaf::new().on_message(MessageCtx::request_rebuild));

        let ctx = message(&mut element, &mut render, &[], Box::new(7_u32));

        assert!(ctx.rebuild_requested(), "leaf called request_rebuild");
    }

    #[test]
    #[should_panic(expected = "message has already been consumed")]
    fn consume_twice_panics() {
        let mut ctx = MessageCtx::new(Box::new(1_u32));
        let _: u32 = ctx.consume();
        let _: u32 = ctx.consume();
    }

    #[test]
    #[should_panic(expected = "message downcast failed")]
    fn consume_wrong_type_panics() {
        let mut ctx = MessageCtx::new(Box::new(1_u32));
        let _: u64 = ctx.consume();
    }

    #[test]
    fn transparent_forwards_path_verbatim() {
        let count = counter();
        let payload = Rc::new(Cell::new(None));
        let (mut element, mut render) = mount(Transparent {
            child: recording_leaf(&count, &payload),
        });

        message(&mut element, &mut render, &[], Box::new(5_u32));

        assert_eq!(count.get(), 1);
        assert_eq!(payload.get(), Some(5));
    }

    #[test]
    fn nested_transparent_wrappers_forward_path_verbatim() {
        let count = counter();
        let payload = Rc::new(Cell::new(None));
        let (mut element, mut render) = mount(Transparent {
            child: Transparent {
                child: recording_leaf(&count, &payload),
            },
        });

        message(&mut element, &mut render, &[], Box::new(11_u32));

        assert_eq!(count.get(), 1);
        assert_eq!(payload.get(), Some(11));
    }

    #[test]
    fn multichild_routes_to_correct_child_by_id() {
        let (m0, m1, m2) = (counter(), counter(), counter());
        let payload = Rc::new(Cell::new(None));
        let (mut element, mut render) = mount(MultiChild {
            children: vec![
                counting_leaf(&m0),
                counting_leaf(&m1),
                recording_leaf(&m2, &payload),
            ],
        });

        message(
            &mut element,
            &mut render,
            &[RoutingId::new(2)],
            Box::new(99_u32),
        );

        assert_eq!((m0.get(), m1.get(), m2.get()), (0, 0, 1));
        assert_eq!(payload.get(), Some(99));
    }

    #[test]
    fn multichild_rebuild_only_touches_target_child() {
        let (r0, r1) = (counter(), counter());
        let (mut element, mut render) = mount(MultiChild {
            children: vec![rebuilding_leaf(&r0), rebuilding_leaf(&r1)],
        });

        rebuild(&mut element, &mut render, &[RoutingId::new(0)]);

        assert_eq!((r0.get(), r1.get()), (1, 0));
    }

    #[test]
    fn dispatch_through_transparent_then_routing_widget() {
        let (m0, m1) = (counter(), counter());
        let payload = Rc::new(Cell::new(None));
        let (mut element, mut render) = mount(Transparent {
            child: MultiChild {
                children: vec![counting_leaf(&m0), recording_leaf(&m1, &payload)],
            },
        });

        message(
            &mut element,
            &mut render,
            &[RoutingId::new(1)],
            Box::new(3_u32),
        );

        assert_eq!((m0.get(), m1.get()), (0, 1));
        assert_eq!(payload.get(), Some(3));
    }

    #[test]
    fn deep_path_through_nested_routing_widgets() {
        let (m00, m01, m10, m11) = (counter(), counter(), counter(), counter());
        let payload = Rc::new(Cell::new(None));
        let (mut element, mut render) = mount(MultiChild {
            children: vec![
                MultiChild {
                    children: vec![counting_leaf(&m00), recording_leaf(&m01, &payload)],
                },
                MultiChild {
                    children: vec![counting_leaf(&m10), counting_leaf(&m11)],
                },
            ],
        });

        message(
            &mut element,
            &mut render,
            &[RoutingId::new(0), RoutingId::new(1)],
            Box::new(77_u32),
        );

        assert_eq!(m01.get(), 1);
        assert_eq!(payload.get(), Some(77));
        assert_eq!((m00.get(), m10.get(), m11.get()), (0, 0, 0));
    }

    #[test]
    fn update_through_routing_widget_reaches_each_child() {
        let (u0, u1) = (counter(), counter());
        let (mut element, mut render) = mount(MultiChild {
            children: vec![updating_leaf(&u0), updating_leaf(&u1)],
        });

        with_ctx(|ctx| {
            MultiChild {
                children: vec![updating_leaf(&u0), updating_leaf(&u1)],
            }
            .update(&mut element, &mut render, ctx);
        });

        assert_eq!((u0.get(), u1.get()), (1, 1));
    }

    #[test]
    fn dispatch_after_update_reaches_correct_child() {
        let count = counter();
        let payload = Rc::new(Cell::new(None));
        let (mut element, mut render) = mount(MultiChild {
            children: vec![Leaf::new(), recording_leaf(&count, &payload)],
        });

        with_ctx(|ctx| {
            MultiChild {
                children: vec![Leaf::new(), recording_leaf(&count, &payload)],
            }
            .update(&mut element, &mut render, ctx);
        });

        message(
            &mut element,
            &mut render,
            &[RoutingId::new(1)],
            Box::new(55_u32),
        );

        assert_eq!(count.get(), 1);
        assert_eq!(payload.get(), Some(55));
    }
}
