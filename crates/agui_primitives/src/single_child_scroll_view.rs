use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_render::prelude::{element::*, render_object::*};

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

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderSingleChildScrollView {
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
        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

pub struct RenderSingleChildScrollView<Child> {
    child: RenderNode<Child, Option<Size>>,
}

impl<Child> SingleChildRenderObject for RenderSingleChildScrollView<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        f(&self.child.object)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child> RenderObject for RenderSingleChildScrollView<Child>
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
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child> RenderBox for RenderSingleChildScrollView<Child>
where
    Child: RenderBox,
{
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
            .as_ref()
            .expect("child has not been laid out");

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
    use agui_render::test_harness::TestCtx;

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn requires_child_with_intrinsic_width() {
        let render_object = TestCtx::new().laid_out(
            SingleChildScrollView::new(SizedBox::new().width(10)),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(10, 0)),
            "should only be the width of the child"
        );

        let render_object = TestCtx::new().laid_out(
            SingleChildScrollView::new(SizedBox::new().width(256)),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 0)),
            "should not exceed the width of the constraints"
        );

        let render_object = TestCtx::new().laid_out(
            SingleChildScrollView::new(SizedBox::new().width(10).height(16)),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(10, 16)),
            "should be the width of the child and the height of the child"
        );

        let render_object = TestCtx::new().laid_out(
            SingleChildScrollView::new(SizedBox::new().expand_width().height(16)),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 16)),
            "should not exceed the width of the constraints and be the height of the child"
        );

        let render_object = TestCtx::new().laid_out(
            SingleChildScrollView::new(SizedBox::new().width(256).height(256)),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 256)),
            "width is clamped to the constraints but height is left unbounded to scroll"
        );
    }
}

#[cfg(test)]
mod harness {

    use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

    use super::SingleChildScrollView;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_element_lifecycle() {
        ElementLifecycleCheck::new().single_child(SingleChildScrollView::new);
    }

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(|| SingleChildScrollView::new(SizedBox::new().width(10).height(256)));
    }
}
