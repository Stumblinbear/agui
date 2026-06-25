use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

/// A box that lets its child grow unbounded along the vertical axis so it can be scrolled, while taking the
/// child's width clamped to the incoming constraints. It does not itself scroll or clip; it gives the child an
/// unbounded height so a scrolling ancestor can offset it.
#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct SingleChildScrollView<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    #[builder(finish_fn)]
    child: Child,
}

impl<Child> SingleChildScrollView<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    pub fn new(child: Child) -> Self {
        Self::builder().child(child)
    }
}

impl<Child> Widget for SingleChildScrollView<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderSingleChildScrollView<Child::Render>>;

    type Render = RenderSingleChildScrollView<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderSingleChildScrollView {
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.update(ctx, self.child);
    }
}

pub struct RenderSingleChildScrollView<Child: ?Sized> {
    child: RenderNode<Child, Option<Size>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderSingleChildScrollView<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderSingleChildScrollView<Child> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderSingleChildScrollView<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.constrain(self.child.measure(constraints.only_width()))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        let child_size = self
            .child
            .layout_and_get_size(ctx, constraints.only_width());

        self.child.parent_data = Some(child_size);

        constraints.constrain(child_size)
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        let child_size = self
            .child
            .parent_data
            .expect("scroll view child has not been laid out");

        if !child_size.contains(position) {
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

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        test_fixtures::RecordingBox,
        test_harness::{RawWidget, TestCtx},
    };

    use super::*;

    #[test]
    fn lays_its_child_out_with_the_width_constraints_and_an_unbounded_height() {
        let probe = RecordingBox::new();
        let laid_out = Rc::clone(&probe.laid_out);

        let (mut owner, view) =
            TestCtx::new().mount_view(SingleChildScrollView::new(RawWidget::new(probe)));
        view.resize(BoxConstraints::new(16, 128, 32, 128));
        owner.flush_layout();

        // The child takes the smallest size its constraints allow, so its width is the minimum the scroll
        // view passed through and its height is 0: the scroll view dropped the height bound to leave it free
        // to scroll.
        assert_eq!(laid_out.get(), Some(Size::new(16, 0)));
    }
}
