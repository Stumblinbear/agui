use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::prelude::{element::*, render_object::*};

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

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderPadding {
                padding: EdgeInsets {
                    left: self.padding.left(self.text_direction),
                    top: self.padding.top(),
                    right: self.padding.right(self.text_direction),
                    bottom: self.padding.bottom(),
                },

                layout_scope: LayoutScope::detached(),

                child: RenderNode::new(child_render),
            },
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        // TODO(trevin): mark for re-layout if padding changes
        let new_padding = EdgeInsets {
            left: self.padding.left(self.text_direction),
            top: self.padding.top(),
            right: self.padding.right(self.text_direction),
            bottom: self.padding.bottom(),
        };

        if render_object.padding != new_padding {
            render_object.padding = new_padding;

            ctx.mark_needs_layout(render_object.layout_scope);
        }

        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

#[derive(Debug, PartialEq)]
struct ChildParentData {
    size: Size,
    offset: Offset,
}

pub struct RenderPadding<Child> {
    padding: EdgeInsets,

    layout_scope: LayoutScope,

    child: RenderNode<Child, Option<ChildParentData>>,
}

impl<Child> SingleChildRenderObject for RenderPadding<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        f(&self.child.object)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child> RenderObject for RenderPadding<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.unmount(ctx);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("padding", self.padding)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child> RenderBox for RenderPadding<Child>
where
    Child: RenderBox,
{
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

#[cfg(test)]
mod tests {
    use agui_core::{geometry::EdgeInsets, test_harness::TestCtx};

    use super::*;
    use crate::sized_box::SizedBox;

    #[test]
    fn adds_correct_padding() {
        let mut tcx = TestCtx::new();

        let (_, mut render_object) = tcx.create(Padding::new(EdgeInsets::all(10.0)).child(()));
        let size = render_object.layout(&mut tcx.layout_ctx(), BoxConstraints::new(0, 128, 0, 128));
        assert_eq!(
            size,
            Size::new(20.0, 20.0),
            "padding inflates a zero-size child to the insets"
        );
        assert_eq!(
            render_object
                .child
                .parent_data
                .as_ref()
                .map(|data| data.offset),
            Some(Offset::new(10.0, 10.0)),
            "child is offset by the leading padding"
        );

        let (_, mut render_object) =
            tcx.create(Padding::new(EdgeInsets::all(50.0)).child(SizedBox::shrink()));
        let size = render_object.layout(&mut tcx.layout_ctx(), BoxConstraints::new(0, 128, 0, 128));
        assert_eq!(
            size,
            Size::new(100.0, 100.0),
            "padding inflates a shrunk child to the insets"
        );
        assert_eq!(
            render_object
                .child
                .parent_data
                .as_ref()
                .map(|data| data.offset),
            Some(Offset::new(50.0, 50.0)),
            "child is offset by the leading padding"
        );

        let (_, mut render_object) =
            tcx.create(Padding::new(EdgeInsets::all(50.0)).child(SizedBox::expand()));
        let size = render_object.layout(&mut tcx.layout_ctx(), BoxConstraints::new(0, 128, 0, 128));
        assert_eq!(
            size,
            Size::new(128.0, 128.0),
            "padding plus an expanding child fills the constraints"
        );
        assert_eq!(
            render_object
                .child
                .parent_data
                .as_ref()
                .map(|data| data.offset),
            Some(Offset::new(50.0, 50.0)),
            "child is offset by the leading padding"
        );
    }
}

#[cfg(test)]
mod harness {
    use agui_core::{geometry::EdgeInsets, prelude::element::Size};
    use agui_test::{ElementLifecycleCheck, fixtures::IntrinsicBox, sizing::BoxSizingCheck};

    use super::Padding;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_element_lifecycle() {
        ElementLifecycleCheck::new()
            .single_child(|child| Padding::new(EdgeInsets::all(8)).child(child));
    }

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(|| Padding::new(EdgeInsets::all(8)).child(SizedBox::new().width(16).height(48)));
    }

    #[test]
    fn shrink_wraps_and_is_extent_independent() {
        BoxSizingCheck::new()
            .shrink_wraps_width()
            .shrink_wraps_height()
            .width_independent_of_height()
            .height_independent_of_width()
            .run(|| Padding::new(EdgeInsets::all(8)).child(SizedBox::new().width(16).height(48)));
    }

    #[test]
    fn combines_a_child_with_known_intrinsics() {
        // A child whose minimum and maximum intrinsics differ, so the parent's combination is
        // exercised with metrics the test controls.
        BoxSizingCheck::new()
            .shrink_wraps_width()
            .shrink_wraps_height()
            .run(|| {
                let child = IntrinsicBox::new(Size::new(40, 20)).min_intrinsic(Size::new(10, 8));
                Padding::new(EdgeInsets::all(8)).child(child)
            });
    }
}
