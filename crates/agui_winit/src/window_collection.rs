use agui_core::{
    element::{RenderObjectBuildContext, RenderObjectUpdateContext},
    render::{
        RenderObjectHitTestContext, RenderObjectImpl, RenderObjectIntrinsicSizeContext,
        RenderObjectLayoutContext,
    },
    unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

#[derive(RenderObjectWidget)]
pub struct WinitWindowCollection {
    windows: Vec<Widget>,
}

impl RenderObjectWidget for WinitWindowCollection {
    type RenderObject = RenderWinitWindowCollection;

    fn children(&self) -> Vec<Widget> {
        self.windows.clone()
    }

    fn create_render_object(&self, ctx: &mut RenderObjectBuildContext) -> Self::RenderObject {
        RenderWinitWindowCollection {}
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
    }
}

pub struct RenderWinitWindowCollection {}

impl RenderObjectImpl for RenderWinitWindowCollection {
    fn intrinsic_size<'ctx>(
        &'ctx self,
        _: &mut RenderObjectIntrinsicSizeContext<'ctx>,
        _: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        0.0
    }

    fn layout<'ctx>(
        &'ctx mut self,
        _: &mut RenderObjectLayoutContext<'ctx>,
        _: Constraints,
    ) -> Size {
        Size::ZERO
    }

    fn hit_test<'ctx>(&self, _: &'ctx mut RenderObjectHitTestContext<'ctx>, _: Offset) -> HitTest {
        HitTest::Pass
    }
}
