use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::Element,
    hit_test::HitTestResult,
    offset::Offset,
    render_object::{HasIntrinsic, RenderObject},
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    view::View,
};

#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct IntrinsicWidth<Child> {
    #[builder(finish_fn)]
    child: Child,
}

impl<Child> View for IntrinsicWidth<Child>
where
    Child: View,
    Child::Render: RenderObject<WidthIntrinsic = HasIntrinsic>,
{
    type Render = RenderIntrinsicWidth<Child::Render>;

    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (vec![Element::new(&self.child, ctx)], ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        element.child_mut(0, &old.child).update(&self.child, ctx);
    }

    fn message(&self, element: &mut Element, ctx: MessageCtx) {
        match ctx.routing_id() {
            Some(0) => element.child_mut(0, &self.child).message(ctx),
            _ => unreachable!(),
        }
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        RenderIntrinsicWidth {
            child: element.child(0, &self.child).create_render_object(),
        }
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        element
            .child(0, &self.child)
            .update_render_object(&mut render_object.child);
    }
}

pub struct RenderIntrinsicWidth<Child> {
    child: Child,
}

impl<Child> RenderObject for RenderIntrinsicWidth<Child>
where
    Child: RenderObject<WidthIntrinsic = HasIntrinsic>,
{
    type Width = Child::Width;
    type Height = Child::Height;

    type WidthIntrinsic = HasIntrinsic;
    type HeightIntrinsic = Child::HeightIntrinsic;

    fn mount(&mut self, _: &mut UpdateCtx) {}

    fn unmount(&mut self, _: &mut UpdateCtx) {}

    fn size(&self) -> Size {
        self.child.size()
    }

    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, mut width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        if width.is_finite() {
            width = self
                .max_intrinsic_width(width)
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                )
                .into();
        }

        self.child.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, mut width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        if width.is_finite() {
            width = self
                .max_intrinsic_width(width)
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                )
                .into();
        }

        self.child.max_intrinsic_height(width)
    }

    fn measure(&self, mut constraints: Constraints) -> Size {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(
                self
                .max_intrinsic_width(constraints.max_height())
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                ).get());
        } else {
            // Technically IntrinsicWidth isn't necessary if we're given a tight constraint.
            // Do we want to log anything here? It's not an error, but it could be good
            // to know if this is happening.
        }

        self.child.measure(constraints)
    }

    fn layout(&mut self, mut constraints: Constraints) {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(self
                .max_intrinsic_width(constraints.max_height())
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                ).get());
        } else {
            // Technically IntrinsicWidth isn't necessary if we're given a tight constraint.
            // Do we want to log anything here? It's not an error, but it could be good
            // to know if this is happening.
        }

        self.child.layout(constraints);
    }

    fn measure_baseline(
        &self,
        mut constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(self
                .max_intrinsic_width(constraints.max_height())
                .expect(
                    "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                ).get());
        } else {
            // Technically IntrinsicWidth isn't necessary if we're given a tight constraint.
            // Do we want to log anything here? It's not an error, but it could be good
            // to know if this is happening.
        }

        self.child.measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        if !self.child.size().contains(position) {
            return false;
        }

        self.child.hit_test(result, position)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        self.child.draw(canvas);
    }
}
