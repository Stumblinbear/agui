use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Padding<EdgeGeometry, Child> {
    #[builder(finish_fn)]
    child: Child,

    padding: EdgeGeometry,

    #[builder(default)]
    text_direction: TextDirection,
}

impl<EdgeGeometry, Child> Padding<EdgeGeometry, Child> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        padding: EdgeGeometry,
    ) -> PaddingBuilder<EdgeGeometry, Child, padding_builder::SetPadding> {
        Self::builder().padding(padding)
    }
}

impl<EdgeGeometry, Child> Widget for Padding<EdgeGeometry, Child>
where
    EdgeGeometry: EdgeInsetsGeometry,
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderPadding<Child::Render>>;

    type Render = RenderPadding<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderPadding {
                padding: EdgeInsets {
                    left: self.padding.left(self.text_direction),
                    top: self.padding.top(),
                    right: self.padding.right(self.text_direction),
                    bottom: self.padding.bottom(),
                },

                layout_scope: LayoutScope::detached(),
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();

        let new_padding = EdgeInsets {
            left: self.padding.left(self.text_direction),
            top: self.padding.top(),
            right: self.padding.right(self.text_direction),
            bottom: self.padding.bottom(),
        };

        if render.padding != new_padding {
            render.padding = new_padding;

            ctx.mark_needs_layout(render.layout_scope);
        }

        element.update(ctx, self.child);
    }
}

#[derive(Debug, PartialEq)]
struct ChildParentData {
    size: Size,
    offset: Offset,
}

pub struct RenderPadding<Child: ?Sized> {
    padding: EdgeInsets,

    layout_scope: LayoutScope,
    child: RenderNode<Child, Option<ChildParentData>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderPadding<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderPadding<Child> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("padding", self.padding)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderPadding<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let child_height = self.padding.deflate_axis(Axis::Vertical, height);

        let child_width = self.child.min_intrinsic_width(child_height)?;

        Some(
            self.padding
                .inflate_axis(Axis::Horizontal, child_width.into())
                .try_into()
                .expect("minimum intrinsic width of padding must be finite"),
        )
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let child_height = self.padding.deflate_axis(Axis::Vertical, height);

        let child_width = self.child.max_intrinsic_width(child_height)?;

        Some(
            self.padding
                .inflate_axis(Axis::Horizontal, child_width.into())
                .try_into()
                .expect("maximum intrinsic width of padding must be finite"),
        )
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let child_width = self.padding.deflate_axis(Axis::Horizontal, width);

        let child_height = self.child.min_intrinsic_height(child_width)?;

        Some(
            self.padding
                .inflate_axis(Axis::Vertical, child_height.into())
                .try_into()
                .expect("minimum intrinsic height of padding must be finite"),
        )
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let child_width = self.padding.deflate_axis(Axis::Horizontal, width);

        let child_height = self.child.max_intrinsic_height(child_width)?;

        Some(
            self.padding
                .inflate_axis(Axis::Vertical, child_height.into())
                .try_into()
                .expect("maximum intrinsic height of padding must be finite"),
        )
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        let inner_constraints = constraints.deflate(&self.padding);
        let child_size = self.child.measure(inner_constraints);

        constraints.constrain(self.padding.inflate_size(child_size))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        let inner_constraints = constraints.deflate(&self.padding);
        let child_size = self.child.layout_and_get_size(ctx, inner_constraints);

        self.child.parent_data = Some(ChildParentData {
            size: child_size,
            offset: self.padding.top_left(),
        });

        constraints.constrain(self.padding.inflate_size(child_size))
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        let inner_constraints = constraints.deflate(&self.padding);
        let child_baseline = self.child.measure_baseline(inner_constraints, baseline)?;

        Some(
            self.padding
                .inflate_axis(Axis::Vertical, child_baseline.into())
                .try_into()
                .expect("baseline of padding must be finite"),
        )
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        let child_offset = self
            .child
            .parent_data
            .as_ref()
            .expect("child has not been laid out")
            .offset;

        self.child.distance_to_baseline(baseline).map(|distance| {
            PositiveFinite::try_from(distance + child_offset.y)
                .expect("distance to baseline of padding was not a positive finite number")
        })
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        let ChildParentData { size, offset } = self
            .child
            .parent_data
            .as_ref()
            .expect("child has not been laid out");

        if !size.contains(position) {
            return HitTest::Pass;
        }

        result.with_offset(*offset, position, |result, transformed| {
            self.child.hit_test(result, transformed)
        })
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(
            ctx,
            offset + Offset::new(self.padding.left, self.padding.top),
        );
    }
}
