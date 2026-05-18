use std::{
    any::{Any, TypeId},
    mem::ManuallyDrop,
};

use crate::{
    context::{Dispatch, UpdateCtx},
    element::{Element, ElementState},
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

#[diagnostic::on_unimplemented(
    message = "Trait bound View is not satisfied.",
    note = "dyn View is not supported, use dyn AnyView via .as_dyn_view() or .into_boxed_view() instead."
)]
pub trait MountView {
    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState);
}

impl<T> MountView for T
where
    T: View,
{
    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState) {
        let (children, state) = <T as View>::mount(self, ctx);

        if TypeId::of::<T::State>() == TypeId::of::<ElementState>()
            && size_of::<T::State>() == size_of::<ElementState>()
        {
            // Since this is an owned value, we need to mark it as a manually dropped value so that
            // it doesn't get immediately dropped when we return it after transmuting it.
            let state = ManuallyDrop::new(state);

            // SAFETY: This is probably safe so long as there are no TypeId + size collisions
            let state = unsafe { std::mem::transmute_copy::<T::State, ElementState>(&state) };

            return (children, state);
        }

        (children, ElementState::new(state))
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
        context::{Dispatch, MessageCtx, UpdateCtx},
        element::Element,
        render_object::RenderLeaf,
        routing_id::RoutingId,
        test_harness::TestHarness,
        view::View,
    };

    struct Recorder {
        message_calls: Cell<usize>,
        rebuild_calls: Cell<usize>,
        last_payload: Cell<Option<u32>>,
        request_rebuild_on_message: bool,
    }

    impl Recorder {
        fn new() -> Self {
            Self {
                message_calls: Cell::new(0),
                rebuild_calls: Cell::new(0),
                last_payload: Cell::new(None),
                request_rebuild_on_message: false,
            }
        }

        fn requesting_rebuild() -> Self {
            Self {
                request_rebuild_on_message: true,
                ..Self::new()
            }
        }
    }

    impl View for Recorder {
        type Render = RenderLeaf;
        type State = ();

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (Vec::new(), ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn dispatch(
            &self,
            _element: &mut Element,
            path: &[crate::routing_id::RoutingId],
            action: Dispatch,
        ) {
            debug_assert!(path.is_empty(), "Recorder is a leaf");

            if !path.is_empty() {
                return;
            }

            match action {
                Dispatch::Message(ctx) => {
                    self.message_calls.set(self.message_calls.get() + 1);
                    self.last_payload.set(Some(ctx.consume::<u32>()));
                    if self.request_rebuild_on_message {
                        ctx.request_rebuild();
                    }
                }
                Dispatch::Rebuild(_ctx) => {
                    self.rebuild_calls.set(self.rebuild_calls.get() + 1);
                }
            }
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn message_at_empty_path_invokes_leaf() {
        let view = Recorder::new();
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[], Box::new(42_u32));

        assert_eq!(view.message_calls.get(), 1);
        assert_eq!(view.rebuild_calls.get(), 0);
        assert_eq!(view.last_payload.get(), Some(42));
    }

    #[test]
    fn rebuild_at_empty_path_invokes_leaf() {
        let view = Recorder::new();
        let mut harness = TestHarness::mount(&view);

        harness.dispatch_rebuild(&view, &[]);

        assert_eq!(view.message_calls.get(), 0);
        assert_eq!(view.rebuild_calls.get(), 1);
    }

    #[test]
    fn message_without_request_rebuild_leaves_flag_unset() {
        let view = Recorder::new();
        let mut harness = TestHarness::mount(&view);

        let msg_ctx = harness.dispatch_message(&view, &[], Box::new(1_u32));

        assert!(
            !msg_ctx.rebuild_requested(),
            "leaf did not call request_rebuild"
        );
    }

    #[test]
    fn message_with_request_rebuild_sets_flag() {
        let view = Recorder::requesting_rebuild();
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

    struct Transparent<Child> {
        child: Child,
    }

    impl<Child> View for Transparent<Child>
    where
        Child: View,
    {
        type Render = RenderLeaf;
        type State = ();

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![Element::new(&self.child, ctx)], ())
        }

        fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
            element.child_mut(0, &old.child).update(&self.child, ctx);
        }

        fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
            element.child_mut(0, &self.child).dispatch(path, action)
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    struct MultiChild<Child> {
        children: Vec<Child>,
    }

    impl<Child> View for MultiChild<Child>
    where
        Child: View,
    {
        type Render = RenderLeaf;
        type State = ();

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            let children = self
                .children
                .iter()
                .enumerate()
                .map(|(idx, child)| {
                    ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| Element::new(child, ctx))
                })
                .collect();
            (children, ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
            let Some((head, rest)) = path.split_first() else {
                return;
            };

            let idx = head.get() as usize;

            element
                .child_mut(idx, &self.children[idx])
                .dispatch(rest, action)
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn transparent_forwards_path_verbatim() {
        // Transparent does not consume any path entry. Dispatching with an empty
        // path through Transparent reaches the inner leaf directly.
        let view = Transparent {
            child: Recorder::new(),
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[], Box::new(5_u32));

        assert_eq!(view.child.message_calls.get(), 1);
        assert_eq!(view.child.last_payload.get(), Some(5));
    }

    #[test]
    fn nested_transparent_wrappers_forward_path_verbatim() {
        // Two layers of Transparent should still expose the inner leaf at empty
        // path; the path passes through both layers unchanged.
        let view = Transparent {
            child: Transparent {
                child: Recorder::new(),
            },
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[], Box::new(11_u32));

        assert_eq!(view.child.child.message_calls.get(), 1);
        assert_eq!(view.child.child.last_payload.get(), Some(11));
    }

    #[test]
    fn multichild_routes_to_correct_child_by_id() {
        // MultiChild pushes routing id N for child N. Dispatching `[2]` reaches
        // the third child and no others.
        let view = MultiChild {
            children: vec![Recorder::new(), Recorder::new(), Recorder::new()],
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[RoutingId::new(2)], Box::new(99_u32));

        assert_eq!(view.children[0].message_calls.get(), 0);
        assert_eq!(view.children[1].message_calls.get(), 0);
        assert_eq!(view.children[2].message_calls.get(), 1);
        assert_eq!(view.children[2].last_payload.get(), Some(99));
    }

    #[test]
    fn multichild_rebuild_only_touches_target_child() {
        // Dispatch::Rebuild through a routing view should only invoke `rebuild`
        // on the addressed child, not on siblings.
        let view = MultiChild {
            children: vec![Recorder::new(), Recorder::new()],
        };
        let mut harness = TestHarness::mount(&view);

        harness.dispatch_rebuild(&view, &[RoutingId::new(0)]);

        assert_eq!(view.children[0].rebuild_calls.get(), 1);
        assert_eq!(view.children[1].rebuild_calls.get(), 0);
    }

    #[test]
    fn dispatch_through_transparent_then_routing_view() {
        // A path like `[1]` should pass through a transparent outer view and
        // then index into the multichild beneath it.
        let view = Transparent {
            child: MultiChild {
                children: vec![Recorder::new(), Recorder::new()],
            },
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(&view, &[RoutingId::new(1)], Box::new(3_u32));

        assert_eq!(view.child.children[0].message_calls.get(), 0);
        assert_eq!(view.child.children[1].message_calls.get(), 1);
        assert_eq!(view.child.children[1].last_payload.get(), Some(3));
    }

    #[test]
    fn deep_path_through_nested_routing_views() {
        // Path `[0, 1]` walks into MultiChild's slot 0, then into that nested
        // MultiChild's slot 1.
        let view = MultiChild {
            children: vec![
                MultiChild {
                    children: vec![Recorder::new(), Recorder::new()],
                },
                MultiChild {
                    children: vec![Recorder::new(), Recorder::new()],
                },
            ],
        };
        let mut harness = TestHarness::mount(&view);

        let _ = harness.dispatch_message(
            &view,
            &[RoutingId::new(0), RoutingId::new(1)],
            Box::new(77_u32),
        );

        assert_eq!(view.children[0].children[0].message_calls.get(), 0);
        assert_eq!(view.children[0].children[1].message_calls.get(), 1);
        assert_eq!(view.children[0].children[1].last_payload.get(), Some(77));
        assert_eq!(view.children[1].children[0].message_calls.get(), 0);
        assert_eq!(view.children[1].children[1].message_calls.get(), 0);
    }
}
