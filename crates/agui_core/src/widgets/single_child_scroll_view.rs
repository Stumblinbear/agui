use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::{Element, ElementState},
    hit_test::HitTestResult,
    offset::Offset,
    size::Size,
    text_baseline::TextBaseline,
    view::{Bounded, View, ViewDraw, ViewLayout, ViewLayoutConstraints, ViewLifecycle},
};

#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct SingleChildScrollView<Child>
where
    Child: ViewLayoutConstraints<Height = Bounded>,
{
    #[builder(finish_fn)]
    child: Child,
}

impl<Child> ViewLifecycle for SingleChildScrollView<Child>
where
    Child: ViewLifecycle,
    Child: ViewLayoutConstraints<Height = Bounded>,
{
    fn state(&self) -> ElementState {
        ElementState::none()
    }

    fn children(&self) -> Vec<Element> {
        vec![Element::new(&self.child)]
    }

    fn update(&self, element: &mut Element, ctx: UpdateCtx) {
        element.child_mut(0, &self.child).update(ctx);
    }

    fn message(&self, element: &mut Element, ctx: MessageCtx) {
        match ctx.routing_id() {
            Some(0) => element.child_mut(0, &self.child).message(ctx),
            _ => unreachable!(),
        }
    }
}

impl<Child> ViewLayoutConstraints for SingleChildScrollView<Child>
where
    Child: ViewLayout,
    Child: ViewLayoutConstraints<Height = Bounded>,
{
    type Width = Bounded;
    type Height = Bounded;
}

impl<Child> ViewLayout for SingleChildScrollView<Child>
where
    Child: ViewLayout,
    Child: ViewLayoutConstraints<Height = Bounded>,
{
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
}
impl<Renderer, Child> ViewDraw<Renderer> for SingleChildScrollView<Child>
where
    Renderer: crate::renderer::Renderer,
    Child: View<Renderer>,
    Child: ViewLayout,
    Child: ViewLayoutConstraints<Height = Bounded>,
{
    fn draw(&self, element: &mut Element, renderer: &mut Renderer) {
        element.child_mut(0, &self.child).draw(renderer);
    }
}

#[cfg(test)]
mod tests {
    use crate::widgets::sized_box::SizedBox;

    use super::*;

    #[test]
    fn intrinsic_width() {
        let mut element = Element::empty();

        let scroll_view = SingleChildScrollView::new().child(SizedBox::new().width(10));
        element.update(&scroll_view);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(10, 0),
            "should only be the width of the child"
        );

        let scroll_view = SingleChildScrollView::new().child(SizedBox::new().width(256));
        element.update(&scroll_view);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(128, 0),
            "should not exceed the width of the constraints"
        );

        let scroll_view = SingleChildScrollView::new().child(SizedBox::new().width(10).height(16));
        element.update(&scroll_view);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(10, 16),
            "should be the width of the child and the height of the child"
        );

        let scroll_view =
            SingleChildScrollView::new().child(SizedBox::new().expand_width().height(16));
        element.update(&scroll_view);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(128, 16),
            "should not exceed the width of the constraints and be the height of the child"
        );

        let scroll_view =
            SingleChildScrollView::new().child(SizedBox::new().width(256).height(256));
        element.update(&scroll_view);
        assert_eq!(
            scroll_view.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(128, 128),
            "should not exceed the width or height of the constraints"
        );
    }
}
