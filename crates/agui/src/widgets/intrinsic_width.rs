use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct IntrinsicWidth<Child> {
    #[builder(finish_fn)]
    child: Child,
}

impl<Child> Widget for IntrinsicWidth<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderIntrinsicWidth<Child::Render>>;

    type Render = RenderIntrinsicWidth<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderIntrinsicWidth {
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.update(ctx, self.child);
    }
}

pub struct RenderIntrinsicWidth<Child: ?Sized> {
    child: RenderNode<Child, Option<Size>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderIntrinsicWidth<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderIntrinsicWidth<Child> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderIntrinsicWidth<Child> {
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

    fn measure(&self, mut constraints: BoxConstraints) -> Size {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(
                self.max_intrinsic_width(constraints.max_height())
                    .expect(
                        "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                    )
                    .get(),
            );
        }

        self.child.measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, mut constraints: BoxConstraints) -> Size {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(
                self.max_intrinsic_width(constraints.max_height())
                    .expect(
                        "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                    )
                    .get(),
            );
        }

        let size = self.child.layout_and_get_size(ctx, constraints);
        self.child.parent_data = Some(size);
        size
    }

    fn measure_baseline(
        &self,
        mut constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        if !constraints.has_tight_width() {
            constraints = constraints.tighten_width(
                self.max_intrinsic_width(constraints.max_height())
                    .expect(
                        "IntrinsicWidth must have a child that has a bounded maximum intrinsic width",
                    )
                    .get(),
            );
        }

        self.child.measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        if !self
            .child
            .parent_data
            .expect("child has not been laid out")
            .contains(position)
        {
            return HitTest::Pass;
        }

        self.child.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}
