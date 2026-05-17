use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::Element,
    hit_test::HitTestResult,
    offset::Offset,
    render_object::{
        box_layout::{BoxLayout, RenderBox},
        AsAnyRenderObject, RenderObject,
    },
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    view::View,
};

#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct SingleChildScrollView<Child>
where
    Child: View,
    Child::Render: RenderBox,
{
    #[builder(finish_fn)]
    child: Child,
}

impl<Child> SingleChildScrollView<Child>
where
    Child: View,
    Child::Render: RenderBox,
{
    pub fn new(child: Child) -> Self {
        Self::builder().child(child)
    }
}

impl<Child> View for SingleChildScrollView<Child>
where
    Child: View,
    Child::Render: RenderBox,
{
    type Render = RenderSingleChildScrollView<Child::Render>;

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
        RenderSingleChildScrollView {
            child: element.child(0, &self.child).create_render_object(),

            size: Size::ZERO,
        }
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        element
            .child(0, &self.child)
            .update_render_object(&mut render_object.child);
    }
}
pub struct RenderSingleChildScrollView<Child> {
    child: Child,

    size: Size,
}

impl<Child> RenderObject for RenderSingleChildScrollView<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut UpdateCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx) {
        self.child.unmount(ctx);
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        if !self.size().contains(position) {
            return false;
        }

        self.child.hit_test(result, position)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        self.child.draw(canvas);
    }
}

impl<Child> BoxLayout for RenderSingleChildScrollView<Child>
where
    Child: RenderBox,
{
    fn size(&self) -> Size {
        self.size
    }

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

    fn measure(&self, constraints: Constraints) -> Size {
        self.child.measure(constraints.only_width())
    }

    fn layout(&mut self, constraints: Constraints) {
        self.child.layout(constraints.only_width());

        // TODO(trevin): mark this as dependent on the child size
        self.size = constraints.constrain(self.child.size());
    }

    fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }
}

impl<Child> AsAnyRenderObject for RenderSingleChildScrollView<Child>
where
    Self: RenderBox,
{
    type Output = dyn agui_core::render_object::box_layout::AnyRenderBox;

    fn as_dyn_render_object(&self) -> &dyn agui_core::render_object::AnyRenderObject {
        self
    }

    fn into_boxed_render_object(self) -> Box<Self::Output> {
        Box::new(self)
    }
}

#[cfg(test)]
mod tests {
    use agui_core::test_harness::TestHarness;

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn requires_child_with_intrinsic_width() {
        let scroll_view = SingleChildScrollView::new(SizedBox::new().width(10));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(10, 0),
            "should only be the width of the child"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().width(256));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(128, 0),
            "should not exceed the width of the constraints"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().width(10).height(16));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(10, 16),
            "should be the width of the child and the height of the child"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().expand_width().height(16));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(128, 16),
            "should not exceed the width of the constraints and be the height of the child"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().width(256).height(256));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(128, 128),
            "should not exceed the width or height of the constraints"
        );
    }
}
