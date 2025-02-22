use bon::Builder;
use typed_floats::{as_const, Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    edge_insets::EdgeInsetsGeometry,
    element::Element,
    hit_test::HitTestResult,
    offset::Offset,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    text_direction::TextDirection,
    view::{View, ViewLayoutConstraints},
};

#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct Padding<EdgeGeometry, Child> {
    #[builder(start_fn)]
    padding: EdgeGeometry,

    #[builder(finish_fn)]
    child: Child,

    #[builder(default)]
    text_direction: TextDirection,
}

#[derive(Default)]
#[doc(hidden)]
pub struct PaddingState {
    child_offset: Offset,
}

impl<EdgeGeometry, Child> ViewLayoutConstraints for Padding<EdgeGeometry, Child>
where
    Child: View,
{
    type Width = Child::Width;
    type Height = Child::Height;
}

impl<EdgeGeometry, Child> View for Padding<EdgeGeometry, Child>
where
    EdgeGeometry: EdgeInsetsGeometry,
    Child: View,
{
    type State = PaddingState;

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (
            vec![Element::new(&self.child, ctx)],
            PaddingState::default(),
        )
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
        let inner_height = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(height - self.padding.vertical())
                    .get(),
            )
        };

        element
            .child(0, &self.child)
            .min_intrinsic_width(inner_height)
            .map(|width| {
                PositiveFinite::try_from(width + self.padding.horizontal())
                    .expect("minimum intrinsic width of padding must be finite")
            })
    }

    fn max_intrinsic_width(
        &self,
        element: &Element,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        let inner_height = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(height - self.padding.vertical())
                    .get(),
            )
        };

        element
            .child(0, &self.child)
            .max_intrinsic_width(inner_height)
            .map(|width| {
                PositiveFinite::try_from(width + self.padding.horizontal())
                    .expect("minimum intrinsic width of padding must be finite")
            })
    }

    fn min_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        let inner_width = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(width - self.padding.horizontal())
                    .get(),
            )
        };

        element
            .child(0, &self.child)
            .min_intrinsic_height(inner_width)
            .map(|height| {
                PositiveFinite::try_from(height + self.padding.vertical())
                    .expect("minimum intrinsic height of padding must be finite")
            })
    }

    fn max_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        let inner_width = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(width - self.padding.horizontal())
                    .get(),
            )
        };

        element
            .child(0, &self.child)
            .max_intrinsic_height(inner_width)
            .map(|height| {
                PositiveFinite::try_from(height + self.padding.vertical())
                    .expect("minimum intrinsic height of padding must be finite")
            })
    }

    fn measure(&self, element: &Element, constraints: Constraints) -> Size {
        let inner_constraints = constraints.deflate(&self.padding);

        let child_size = element.child(0, &self.child).measure(inner_constraints);

        constraints
            .constrain(Size::new(self.padding.horizontal(), self.padding.vertical()) + child_size)
    }

    fn layout(&self, element: &mut Element, constraints: Constraints) -> Size {
        let inner_constraints = constraints.deflate(&self.padding);

        let child_size = element
            .child_mut(0, &self.child)
            .layout(inner_constraints)
            .size();

        element.state_mut::<Self>().child_offset =
            Offset::new(self.padding.left(self.text_direction), self.padding.top());

        constraints
            .constrain(Size::new(self.padding.horizontal(), self.padding.vertical()) + child_size)
    }

    fn measure_baseline(
        &self,
        element: &Element,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        let inner_constraints = constraints.deflate(&self.padding);

        element
            .child(0, &self.child)
            .measure_baseline(inner_constraints, baseline)
            .map(|baseline| {
                PositiveFinite::try_from(baseline + self.padding.top())
                    .expect("baseline of padding must be finite")
            })
    }

    fn distance_to_baseline(
        &self,
        element: &mut Element,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        element
            .child_mut(0, &self.child)
            .distance_to_baseline(baseline)
            .map(|distance| {
                PositiveFinite::try_from(distance + element.state::<Self>().child_offset.y)
                    .expect("distance to baseline of padding was not a positive finite number")
            })
    }

    fn hit_test(&self, element: &Element, result: &mut HitTestResult, position: Offset) -> bool {
        if !element.size().contains(position) {
            return false;
        }

        result.with_offset(
            element.state::<Self>().child_offset,
            position,
            |result, transformed| element.child(0, &self.child).hit_test(result, transformed),
        )
    }

    fn draw(&self, element: &mut Element, canvas: &mut Canvas) {
        let left = self.padding.left(self.text_direction);
        let top = self.padding.top();

        canvas.with_offset(Offset::new(left, top), |canvas| {
            element.child_mut(0, &self.child).draw(canvas);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::mpsc};

    use super::*;
    use crate::{edge_insets::EdgeInsets, widgets::sized_box::SizedBox};

    #[test]
    fn padding() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        let padding = Padding::new(EdgeInsets::all(10.0)).child(());
        let mut element = Element::new(&padding, &mut update_ctx);
        assert_eq!(
            padding.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(20.0, 20.0)
        );

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::shrink());
        let mut element = Element::new(&padding, &mut update_ctx);
        assert_eq!(
            padding.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(100.0, 100.0)
        );

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::expand());
        let mut element = Element::new(&padding, &mut update_ctx);
        assert_eq!(
            padding.layout(&mut element, Constraints::new(0, 128, 0, 128)),
            Size::new(128.0, 128.0)
        );
    }
}
