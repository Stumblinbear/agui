use crate::{
    context::{Dispatch, UpdateCtx},
    element::Element,
    key::AnyKeyable,
    routing_id::RoutingId,
};

mod any_widget;

pub use any_widget::*;

/// The immutable description of a piece of the tree. A `Widget` materializes and reconciles its
/// persistent [`Element`] (which holds state and children) and supplies the recipe for its render
/// object. The render type lives here, on the description, so the [`Element`] can stay
/// render-agnostic and be shared across widgets.
pub trait Widget {
    type Element: Element;

    type Render;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element;

    /// Reconcile `element` in place against this (new) widget; `old` is the previous widget of the same
    /// type, for prop diffing.
    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx);

    /// Route a [`Dispatch`] along `path` to the destination element.
    fn dispatch(&self, _element: &mut Self::Element, path: &[RoutingId], _action: Dispatch) {
        debug_assert!(path.is_empty(), "widget has nothing to route to");
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render;

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render);

    fn is_same_type(&self, other: &Self) -> bool {
        let _ = other;
        true
    }

    /// This is an implementation detail of element keys and should not be overriden by any user code.
    fn key(&self) -> Option<&dyn AnyKeyable> {
        None
    }
}

impl Widget for () {
    type Element = ();

    type Render = ();

    fn create_element(&self, _: &mut UpdateCtx) {}

    fn update(&self, (): &mut (), (): &Self, _: &mut UpdateCtx) {}

    fn create_render_object(&self, (): &()) -> Self::Render {}

    fn update_render_object(&self, (): &(), (): &mut Self::Render) {}
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
        let widget = Leaf::new().on_message(|ctx| {
            messages.set(messages.get() + 1);
            payload.set(Some(ctx.consume::<u32>()));
        });
        let mut harness = TestHarness::mount(&widget);

        let _ = harness.dispatch_message(&widget, &[], Box::new(42_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(42));
    }

    #[test]
    fn rebuild_at_empty_path_invokes_leaf() {
        let rebuilds = Cell::new(0_usize);
        let widget = Leaf::new().on_rebuild(|_| rebuilds.set(rebuilds.get() + 1));
        let mut harness = TestHarness::mount(&widget);

        harness.dispatch_rebuild(&widget, &[]);

        assert_eq!(rebuilds.get(), 1);
    }

    #[test]
    fn message_without_request_rebuild_leaves_flag_unset() {
        let widget = Leaf::new();
        let mut harness = TestHarness::mount(&widget);

        let msg_ctx = harness.dispatch_message(&widget, &[], Box::new(1_u32));

        assert!(
            !msg_ctx.rebuild_requested(),
            "leaf did not call request_rebuild"
        );
    }

    #[test]
    fn message_with_request_rebuild_sets_flag() {
        let widget = Leaf::new().on_message(MessageCtx::request_rebuild);
        let mut harness = TestHarness::mount(&widget);

        let msg_ctx = harness.dispatch_message(&widget, &[], Box::new(7_u32));

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
        let widget = Transparent {
            child: Leaf::new().on_message(|ctx| {
                messages.set(messages.get() + 1);
                payload.set(Some(ctx.consume::<u32>()));
            }),
        };
        let mut harness = TestHarness::mount(&widget);

        let _ = harness.dispatch_message(&widget, &[], Box::new(5_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(5));
    }

    #[test]
    fn nested_transparent_wrappers_forward_path_verbatim() {
        // Two layers of Transparent should still expose the inner leaf at empty
        // path; the path passes through both layers unchanged.
        let messages = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let widget = Transparent {
            child: Transparent {
                child: Leaf::new().on_message(|ctx| {
                    messages.set(messages.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            },
        };
        let mut harness = TestHarness::mount(&widget);

        let _ = harness.dispatch_message(&widget, &[], Box::new(11_u32));

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
        let widget = MultiChild {
            children: vec![
                Leaf::new().on_message(|_| m0.set(m0.get() + 1)),
                Leaf::new().on_message(|_| m1.set(m1.get() + 1)),
                Leaf::new().on_message(|ctx| {
                    m2.set(m2.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            ],
        };
        let mut harness = TestHarness::mount(&widget);

        let _ = harness.dispatch_message(&widget, &[RoutingId::new(2)], Box::new(99_u32));

        assert_eq!(m0.get(), 0);
        assert_eq!(m1.get(), 0);
        assert_eq!(m2.get(), 1);
        assert_eq!(payload.get(), Some(99));
    }

    #[test]
    fn multichild_rebuild_only_touches_target_child() {
        // Dispatch::Rebuild through a routing widget should only invoke `rebuild`
        // on the addressed child, not on siblings.
        let r0 = Cell::new(0_usize);
        let r1 = Cell::new(0_usize);
        let widget = MultiChild {
            children: vec![
                Leaf::new().on_rebuild(|_| r0.set(r0.get() + 1)),
                Leaf::new().on_rebuild(|_| r1.set(r1.get() + 1)),
            ],
        };
        let mut harness = TestHarness::mount(&widget);

        harness.dispatch_rebuild(&widget, &[RoutingId::new(0)]);

        assert_eq!(r0.get(), 1);
        assert_eq!(r1.get(), 0);
    }

    #[test]
    fn dispatch_through_transparent_then_routing_widget() {
        // A path like `[1]` should pass through a transparent outer widget and
        // then index into the multichild beneath it.
        let m0 = Cell::new(0_usize);
        let m1 = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let widget = Transparent {
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
        let mut harness = TestHarness::mount(&widget);

        let _ = harness.dispatch_message(&widget, &[RoutingId::new(1)], Box::new(3_u32));

        assert_eq!(m0.get(), 0);
        assert_eq!(m1.get(), 1);
        assert_eq!(payload.get(), Some(3));
    }

    #[test]
    fn deep_path_through_nested_routing_widgets() {
        // Path `[0, 1]` walks into MultiChild's slot 0, then into that nested
        // MultiChild's slot 1.
        let m00 = Cell::new(0_usize);
        let m01 = Cell::new(0_usize);
        let m10 = Cell::new(0_usize);
        let m11 = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let widget = MultiChild {
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
        let mut harness = TestHarness::mount(&widget);

        let _ = harness.dispatch_message(
            &widget,
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
    fn update_through_routing_widget_reaches_correct_child() {
        let updates = [Cell::new(0_usize), Cell::new(0_usize)];
        let old_widget = MultiChild {
            children: vec![
                Leaf::new().on_update(|_| updates[0].set(updates[0].get() + 1)),
                Leaf::new().on_update(|_| updates[1].set(updates[1].get() + 1)),
            ],
        };
        let mut harness = TestHarness::mount(&old_widget);

        let new_widget = MultiChild {
            children: vec![
                Leaf::new().on_update(|_| updates[0].set(updates[0].get() + 1)),
                Leaf::new().on_update(|_| updates[1].set(updates[1].get() + 1)),
            ],
        };
        harness.update(&old_widget, &new_widget);

        assert_eq!(updates[0].get(), 1);
        assert_eq!(updates[1].get(), 1);
    }

    #[test]
    fn dispatch_after_update_reaches_correct_child() {
        let messages = Cell::new(0_usize);
        let payload = Cell::new(None::<u32>);
        let old_widget = MultiChild {
            children: vec![
                Leaf::new(),
                Leaf::new().on_message(|ctx| {
                    messages.set(messages.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            ],
        };
        let mut harness = TestHarness::mount(&old_widget);

        let new_widget = MultiChild {
            children: vec![
                Leaf::new(),
                Leaf::new().on_message(|ctx| {
                    messages.set(messages.get() + 1);
                    payload.set(Some(ctx.consume::<u32>()));
                }),
            ],
        };
        harness.update(&old_widget, &new_widget);

        let _ = harness.dispatch_message(&new_widget, &[RoutingId::new(1)], Box::new(55_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(55));
    }
}
