use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    axis::Axis,
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    edge_insets::{EdgeInsets, EdgeInsetsGeometry},
    element::SingleChildElement,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::PaintCtx,
    render_object::{MountCtx, RenderNode, RenderObject, box_layout::RenderBox},
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    text_direction::TextDirection,
    widget::Widget,
};

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
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderPadding<Child::Render>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        SingleChildElement::new(&self.child, ctx)
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.update(&self.child, &old.child, ctx);
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(&self.child, path, action)
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        RenderPadding {
            padding: EdgeInsets {
                left: self.padding.left(self.text_direction),
                top: self.padding.top(),
                right: self.padding.right(self.text_direction),
                bottom: self.padding.bottom(),
            },

            child: RenderNode::new(element.create_render_object(&self.child)),
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        // TODO(trevin): mark for re-layout if padding changes
        render_object.padding = EdgeInsets {
            left: self.padding.left(self.text_direction),
            top: self.padding.top(),
            right: self.padding.right(self.text_direction),
            bottom: self.padding.bottom(),
        };

        element.update_render_object(&self.child, &mut render_object.child.object);
    }
}

#[derive(Debug, PartialEq)]
struct ChildParentData {
    size: Size,
    offset: Offset,
}

pub struct RenderPadding<Child> {
    padding: EdgeInsets,

    child: RenderNode<Child, Option<ChildParentData>>,
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

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
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

    fn measure(&self, constraints: Constraints) -> Size {
        let inner_constraints = constraints.deflate(&self.padding);
        let child_size = self.child.measure(inner_constraints);

        constraints.constrain(self.padding.inflate_size(child_size))
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        let inner_constraints = constraints.deflate(&self.padding);
        let child_size = self.child.layout_and_get_size(inner_constraints);

        self.child.parent_data = Some(ChildParentData {
            size: child_size,
            offset: self.padding.top_left(),
        });

        constraints.constrain(self.padding.inflate_size(child_size))
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
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

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(
            ctx,
            offset + Offset::new(self.padding.left, self.padding.top),
        );
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{edge_insets::EdgeInsets, test_harness::TestHarness};

    use super::*;
    use crate::sized_box::SizedBox;

    #[test]
    fn adds_correct_padding() {
        let padding = Padding::new(EdgeInsets::all(10.0)).child(());

        let mut render_object =
            padding.create_render_object(&TestHarness::mount(&padding).root.element);
        let size = render_object.layout(Constraints::new(0, 128, 0, 128));
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

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::shrink());
        let mut render_object =
            padding.create_render_object(&TestHarness::mount(&padding).root.element);
        let size = render_object.layout(Constraints::new(0, 128, 0, 128));
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

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::expand());
        let mut render_object =
            padding.create_render_object(&TestHarness::mount(&padding).root.element);
        let size = render_object.layout(Constraints::new(0, 128, 0, 128));
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
