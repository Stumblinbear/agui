pub mod flex;
pub mod intrinsic_width;
pub mod layout_builder;
pub mod padding;
pub mod single_child_scroll_view;
pub mod sized_box;
pub mod stack;

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use agui_core::{
        context::{Dispatch, MessageCtx, UpdateCtx},
        element::Element,
        render_object::RenderLeaf,
        routing_id::RoutingId,
        view::View,
    };

    struct TestListener<Child> {
        child: Child,
    }

    #[derive(Default)]
    struct State {
        event_tx: Option<mpsc::Sender<()>>,
    }

    impl<Child> View for TestListener<Child>
    where
        Child: View,
    {
        type Render = RenderLeaf;

        type State = State;

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![Element::new(&self.child, ctx)], State::default())
        }

        fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
            element.state.downcast_mut::<Self>().event_tx = Some(ctx.event_tx());

            element.child_mut(0, &old.child).update(&self.child, ctx);
        }

        fn rebuild(&self, element: &mut Element, ctx: &mut UpdateCtx) {
            element.child_mut(0, &self.child).rebuild(ctx);
        }

        fn message(&self, _: &mut Element, _: &mut MessageCtx) {}

        fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
            element.child_mut(0, &self.child).dispatch(path, action)
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn message_routing() {
        // let (tx, _) = mpsc::channel();
        // let mut path = VecDeque::new();
        // let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        // let listener = TestListener {
        //     child: SizedBox::new(),
        // };

        // let _ = Element::new(&listener, &mut update_ctx);
    }
}
