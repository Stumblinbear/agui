use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::Element,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::{
        AsAnyRenderObject, RenderNode, RenderObject,
        box_layout::{BoxLayout, RenderBox},
    },
    renderer::Canvas,
    routing_id::RoutingId,
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

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        element.child_mut(0, &self.child).dispatch(path, action)
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        RenderSingleChildScrollView {
            child: RenderNode::new(element.child(0, &self.child).create_render_object()),
        }
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        element
            .child(0, &self.child)
            .update_render_object(&mut render_object.child.object);
    }
}

pub struct RenderSingleChildScrollView<Child> {
    child: RenderNode<Child, Option<Size>>,
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

    fn paint(&mut self, canvas: &mut Canvas) {
        self.child.paint(canvas);
    }
}

impl<Child> BoxLayout for RenderSingleChildScrollView<Child>
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

    fn measure(&self, constraints: Constraints) -> Size {
        self.child.measure(constraints.only_width())
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        let child_size = self.child.layout_and_get_size(constraints.only_width());

        self.child.parent_data = Some(child_size);

        constraints.constrain(child_size)
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
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(10, 0)),
            "should only be the width of the child"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().width(256));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 0)),
            "should not exceed the width of the constraints"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().width(10).height(16));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(10, 16)),
            "should be the width of the child and the height of the child"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().expand_width().height(16));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 16)),
            "should not exceed the width of the constraints and be the height of the child"
        );

        let scroll_view = SingleChildScrollView::new(SizedBox::new().width(256).height(256));
        let mut render_object = TestHarness::mount(&scroll_view)
            .root
            .as_ref(&scroll_view)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 128)),
            "should not exceed the width or height of the constraints"
        );
    }
}
