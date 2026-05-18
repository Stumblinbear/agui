use std::any::Any;

use crate::{
    context::{Dispatch, UpdateCtx},
    element::Element,
    key::AnyKeyable,
    render_object::{RenderLeaf, RenderObject},
    routing_id::RoutingId,
};

mod any_view;

pub use any_view::*;

pub trait View {
    type Render: RenderObject;

    type State: Any
    where
        Self: Sized;

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State)
    where
        Self: Sized;

    /// Called when the tree is updated and the `state` in the [`Element`] is of the same type as `Self::State`.
    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx);

    /// Route a [`Dispatch`] along `path` to the destination element.
    fn dispatch(&self, _element: &mut Element, path: &[RoutingId], _action: Dispatch) {
        debug_assert!(path.is_empty(), "view has nothing to route to");
    }

    fn create_render_object(&self, element: &Element) -> Self::Render;

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render);

    fn is_same_type(&self, other: &Self) -> bool {
        let _ = other;
        true
    }

    /// This is an implementation detail of element keys and should not be overriden by any user code.
    fn key(&self) -> Option<&dyn AnyKeyable> {
        None
    }
}

impl View for () {
    type Render = RenderLeaf;

    type State = ();

    fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (Vec::new(), ())
    }

    fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

    fn create_render_object(&self, _: &Element) -> Self::Render {
        RenderLeaf::default()
    }

    fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
}

#[cfg(test)]
mod dispatch_tests {
    use std::cell::Cell;

    use crate::{
        context::MessageCtx,
        routing_id::RoutingId,
        test_fixtures::{Leaf, MultiChild, Transparent},
        test_harness::TestHarness,
    };

    #[test]
    fn message_at_empty_path_invokes_leaf() {
        let messages = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let view = Leaf::new().on_message(|ctx| {
            messages.set(messages.get() + 1);
            payload.set(Some(ctx.consume::<u32>()));
        });
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[], Box::new(42_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(42));
    }

    #[test]
    fn rebuild_at_empty_path_invokes_leaf() {
        let rebuilds = Cell::new(0_usize);
        let view = Leaf::new().on_rebuild(|_| rebuilds.set(rebuilds.get() + 1));
        let mut harness = TestHarness::mount(&view);

        harness.dispatch_rebuild(&view, &[]);

        assert_eq!(rebuilds.get(), 1);
    }

    #[test]
    fn message_without_request_rebuild_leaves_flag_unset() {
        let view = Leaf::new();
        let mut harness = TestHarness::mount(&view);

        let msg_ctx = harness.dispatch_message(&view, &[], Box::new(1_u32));

        assert!(
            !msg_ctx.rebuild_requested(),
            "leaf did not call request_rebuild"
        );
    }

    #[test]
    fn message_with_request_rebuild_sets_flag() {
        let view = Leaf::new().on_message(|ctx| ctx.request_rebuild());
        let mut harness = TestHarness::mount(&view);

        let msg_ctx = harness.dispatch_message(&view, &[], Box::new(7_u32));

        assert!(
            msg_ctx.rebuild_requested(),
            "leaf called request_rebuild on the MessageCtx"
        );
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
        // Transparent does not consume any path entry. Dispatching with an empty
        // path through Transparent reaches the inner leaf directly.
        let messages = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let view = Transparent {
            child: Leaf::new().on_message(|ctx| {
                messages.set(messages.get() + 1);
                payload.set(Some(ctx.consume::<u32>()));
            }),
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[], Box::new(5_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(5));
    }

    #[test]
    fn nested_transparent_wrappers_forward_path_verbatim() {
        // Two layers of Transparent should still expose the inner leaf at empty
        // path; the path passes through both layers unchanged.
        let messages = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let view = Transparent {
            child: Transparent {
                child: Leaf::new().on_message(|ctx| {
                    messages.set(messages.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            },
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[], Box::new(11_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(11));
    }

    #[test]
    fn multichild_routes_to_correct_child_by_id() {
        // MultiChild pushes routing id N for child N. Dispatching `[2]` reaches
        // the third child and no others.
        let m0 = Cell::new(0_usize);
        let m1 = Cell::new(0_usize);
        let m2 = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let view = MultiChild {
            children: vec![
                Leaf::new().on_message(|_| m0.set(m0.get() + 1)),
                Leaf::new().on_message(|_| m1.set(m1.get() + 1)),
                Leaf::new().on_message(|ctx| {
                    m2.set(m2.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            ],
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[RoutingId::new(2)], Box::new(99_u32));

        assert_eq!(m0.get(), 0);
        assert_eq!(m1.get(), 0);
        assert_eq!(m2.get(), 1);
        assert_eq!(payload.get(), Some(99));
    }

    #[test]
    fn multichild_rebuild_only_touches_target_child() {
        // Dispatch::Rebuild through a routing view should only invoke `rebuild`
        // on the addressed child, not on siblings.
        let r0 = Cell::new(0_usize);
        let r1 = Cell::new(0_usize);
        let view = MultiChild {
            children: vec![
                Leaf::new().on_rebuild(|_| r0.set(r0.get() + 1)),
                Leaf::new().on_rebuild(|_| r1.set(r1.get() + 1)),
            ],
        };
        let mut harness = TestHarness::mount(&view);

        harness.dispatch_rebuild(&view, &[RoutingId::new(0)]);

        assert_eq!(r0.get(), 1);
        assert_eq!(r1.get(), 0);
    }

    #[test]
    fn dispatch_through_transparent_then_routing_view() {
        // A path like `[1]` should pass through a transparent outer view and
        // then index into the multichild beneath it.
        let m0 = Cell::new(0_usize);
        let m1 = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let view = Transparent {
            child: MultiChild {
                children: vec![
                    Leaf::new().on_message(|_| m0.set(m0.get() + 1)),
                    Leaf::new().on_message(|ctx| {
                        m1.set(m1.get() + 1);
                        payload.set(Some(ctx.consume::<u32>()));
                    }),
                ],
            },
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[RoutingId::new(1)], Box::new(3_u32));

        assert_eq!(m0.get(), 0);
        assert_eq!(m1.get(), 1);
        assert_eq!(payload.get(), Some(3));
    }

    #[test]
    fn deep_path_through_nested_routing_views() {
        // Path `[0, 1]` walks into MultiChild's slot 0, then into that nested
        // MultiChild's slot 1.
        let m00 = Cell::new(0_usize);
        let m01 = Cell::new(0_usize);
        let m10 = Cell::new(0_usize);
        let m11 = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let view = MultiChild {
            children: vec![
                MultiChild {
                    children: vec![
                        Leaf::new().on_message(|_| m00.set(m00.get() + 1)),
                        Leaf::new().on_message(|ctx| {
                            m01.set(m01.get() + 1);
                            payload.set(Some(ctx.consume::<u32>()));
                        }),
                    ],
                },
                MultiChild {
                    children: vec![
                        Leaf::new().on_message(|_| m10.set(m10.get() + 1)),
                        Leaf::new().on_message(|_| m11.set(m11.get() + 1)),
                    ],
                },
            ],
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(
            &view,
            &[RoutingId::new(0), RoutingId::new(1)],
            Box::new(77_u32),
        );

        assert_eq!(m00.get(), 0);
        assert_eq!(m01.get(), 1);
        assert_eq!(payload.get(), Some(77));
        assert_eq!(m10.get(), 0);
        assert_eq!(m11.get(), 0);
    }

    #[test]
    fn update_through_routing_view_reaches_correct_child() {
        let updates = [Cell::new(0_usize), Cell::new(0_usize)];
        let old_view = MultiChild {
            children: vec![
                Leaf::new().on_update(|_| updates[0].set(updates[0].get() + 1)),
                Leaf::new().on_update(|_| updates[1].set(updates[1].get() + 1)),
            ],
        };
        let mut harness = TestHarness::mount(&old_view);

        let new_view = MultiChild {
            children: vec![
                Leaf::new().on_update(|_| updates[0].set(updates[0].get() + 1)),
                Leaf::new().on_update(|_| updates[1].set(updates[1].get() + 1)),
            ],
        };
        harness.update(&old_view, &new_view);

        assert_eq!(updates[0].get(), 1);
        assert_eq!(updates[1].get(), 1);
    }

    #[test]
    fn dispatch_after_update_reaches_correct_child() {
        let messages = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let old_view = MultiChild {
            children: vec![
                Leaf::new(),
                Leaf::new().on_message(|ctx| {
                    messages.set(messages.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            ],
        };
        let mut harness = TestHarness::mount(&old_view);

        let new_view = MultiChild {
            children: vec![
                Leaf::new(),
                Leaf::new().on_message(|ctx| {
                    messages.set(messages.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            ],
        };
        harness.update(&old_view, &new_view);

        let _ = harness.dispatch_message(&new_view, &[RoutingId::new(1)], Box::new(55_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(55));
    }
}
