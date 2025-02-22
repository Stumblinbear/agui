pub mod column;
pub mod intrinsic_width;
pub mod layout_builder;
pub mod padding;
pub mod single_child_scroll_view;
pub mod sized_box;
pub mod stack;

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        constraints::Constraints,
        context::{MessageCtx, UpdateCtx},
        element::Element,
        hit_test::HitTestResult,
        offset::Offset,
        renderer::Canvas,
        size::Size,
        text_baseline::TextBaseline,
        view::{View, ViewLayoutMarker},
    };

    struct TestListener<Child> {
        child: Child,
    }

    #[derive(Default)]
    struct State {
        event_tx: Option<mpsc::Sender<()>>,
    }

    impl<Child> ViewLayoutMarker for TestListener<Child>
    where
        Child: View,
    {
        type Width = Child::Width;
        type Height = Child::Height;

        type WidthIntrinsic = Child::WidthIntrinsic;
        type HeightIntrinsic = Child::HeightIntrinsic;
    }

    impl<Child> View for TestListener<Child>
    where
        Child: View,
    {
        type State = State;

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![Element::new(&self.child, ctx)], State::default())
        }

        fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
            element.state.downcast_mut::<Self>().event_tx = Some(ctx.event_tx());

            element.child_mut(0, &self.child).update(&old.child, ctx);
        }

        fn message(&self, element: &mut Element, ctx: MessageCtx) {
            match ctx.routing_id() {
                Some(0) => element.child_mut(0, &self.child).message(ctx),
                _ => unreachable!(),
            }
        }

        fn min_intrinsic_width(
            &self,
            element: &Element,
            height: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            element.child(0, &self.child).min_intrinsic_width(height)
        }

        fn max_intrinsic_width(
            &self,
            element: &Element,
            height: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            element.child(0, &self.child).max_intrinsic_width(height)
        }

        fn min_intrinsic_height(
            &self,
            element: &Element,
            width: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            element.child(0, &self.child).min_intrinsic_height(width)
        }

        fn max_intrinsic_height(
            &self,
            element: &Element,
            width: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            element.child(0, &self.child).max_intrinsic_height(width)
        }

        fn measure(&self, element: &Element, constraints: Constraints) -> Size {
            element.child(0, &self.child).measure(constraints)
        }

        fn layout(&self, element: &mut Element, constraints: Constraints) -> Size {
            element.child(0, &self.child).measure(constraints)
        }

        fn measure_baseline(
            &self,
            element: &Element,
            constraints: Constraints,
            baseline: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            element
                .child(0, &self.child)
                .measure_baseline(constraints, baseline)
        }

        fn distance_to_baseline(
            &self,
            element: &mut Element,
            baseline: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            element
                .child_mut(0, &self.child)
                .distance_to_baseline(baseline)
        }

        fn hit_test(
            &self,
            element: &Element,
            result: &mut HitTestResult,
            position: Offset,
        ) -> bool {
            if !element.size().contains(position) {
                return false;
            }

            element.child(0, &self.child).hit_test(result, position)
        }

        fn draw(&self, element: &mut Element, canvas: &mut Canvas) {
            element.child_mut(0, &self.child).draw(canvas)
        }
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
