use bon::Builder;
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
    view::{View, ViewLayoutConstraints},
};

#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct IntrinsicWidth<Child> {
    #[builder(finish_fn)]
    child: Child,
}

impl<Child> ViewLayoutConstraints for IntrinsicWidth<Child>
where
    Child: View,
{
    type Width = Child::Width;
    type Height = Child::Height;
}

impl<Child> View for IntrinsicWidth<Child>
where
    Child: View,
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
        element.child(0, &self.child).max_intrinsic_width(height)
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
        mut width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        if width.is_finite() {
            width = self
                .max_intrinsic_width(element, width)
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                )
                .into();
        }

        element.child(0, &self.child).min_intrinsic_height(width)
    }

    fn max_intrinsic_height(
        &self,
        element: &Element,
        mut width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        if width.is_finite() {
            width = self
                .max_intrinsic_width(element, width)
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                )
                .into();
        }

        element.child(0, &self.child).max_intrinsic_height(width)
    }

    fn measure(&self, element: &Element, mut constraints: Constraints) -> Size {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(self
                .max_intrinsic_width(element, constraints.max_height())
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                ).get());
        } else {
            // Technically IntrinsicWidth isn't necessary if we're given a tight constraint.
            // Do we want to log anything here? It's not an error, but it could be good
            // to know if this is happening.
        }

        element.child(0, &self.child).measure(constraints)
    }

    fn layout(&self, element: &mut Element, mut constraints: Constraints) -> Size {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(self
                .max_intrinsic_width(element, constraints.max_height())
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                ).get());
        } else {
            // Technically IntrinsicWidth isn't necessary if we're given a tight constraint.
            // Do we want to log anything here? It's not an error, but it could be good
            // to know if this is happening.
        }

        element.child_mut(0, &self.child).layout(constraints).size()
    }

    fn measure_baseline(
        &self,
        element: &Element,
        mut constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(self
                .max_intrinsic_width(element, constraints.max_height())
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                ).get());
        } else {
            // Technically IntrinsicWidth isn't necessary if we're given a tight constraint.
            // Do we want to log anything here? It's not an error, but it could be good
            // to know if this is happening.
        }

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
