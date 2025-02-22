use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::Element,
    hit_test::HitTestResult,
    offset::Offset,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    view::{Bounded, View, ViewLayoutMarker},
};

#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct SingleChildScrollView<Child>
where
    Child: ViewLayoutMarker<Height = Bounded>,
{
    #[builder(finish_fn)]
    child: Child,
}

impl<Child> ViewLayoutMarker for SingleChildScrollView<Child>
where
    Child: ViewLayoutMarker<Height = Bounded>,
{
    type Width = Bounded;
    type Height = Bounded;

    // TODO(trevin): should this support intrinsic dimensions?
    type WidthIntrinsic = Child::WidthIntrinsic;
    type HeightIntrinsic = Child::HeightIntrinsic;
}

impl<Child> View for SingleChildScrollView<Child>
where
    Child: View<Height = Bounded>,
{
    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (vec![Element::new(&self.child, ctx)], ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
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
        element
            .child(0, &self.child)
            .measure(constraints.only_width())
    }

    fn layout(&self, element: &mut Element, constraints: Constraints) -> Size {
        let child_size = element
            .child_mut(0, &self.child)
            .layout(constraints.only_width())
            .size();

        constraints.constrain(child_size)
    }

    fn measure_baseline(
        &self,
        _: &Element,
        _: Constraints,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(
        &self,
        _: &mut Element,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, element: &Element, result: &mut HitTestResult, position: Offset) -> bool {
        if !element.size().contains(position) {
            return false;
        }

        element.child(0, &self.child).hit_test(result, position)
    }

    fn draw(&self, element: &mut Element, canvas: &mut Canvas) {
        element.child_mut(0, &self.child).draw(canvas);
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::mpsc};

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn intrinsic_width() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        let scroll_view = SingleChildScrollView::new().child(SizedBox::new().width(10));
        let mut element = Element::new(&scroll_view, &mut update_ctx);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(10, 0),
            "should only be the width of the child"
        );

        let scroll_view = SingleChildScrollView::new().child(SizedBox::new().width(256));
        let mut element = Element::new(&scroll_view, &mut update_ctx);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(128, 0),
            "should not exceed the width of the constraints"
        );

        let scroll_view = SingleChildScrollView::new().child(SizedBox::new().width(10).height(16));
        let mut element = Element::new(&scroll_view, &mut update_ctx);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(10, 16),
            "should be the width of the child and the height of the child"
        );

        let scroll_view =
            SingleChildScrollView::new().child(SizedBox::new().expand_width().height(16));
        let mut element = Element::new(&scroll_view, &mut update_ctx);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(128, 16),
            "should not exceed the width of the constraints and be the height of the child"
        );

        let scroll_view =
            SingleChildScrollView::new().child(SizedBox::new().width(256).height(256));
        let mut element = Element::new(&scroll_view, &mut update_ctx);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(128, 128),
            "should not exceed the width or height of the constraints"
        );
    }
}
