use agui_core::{
    element::{RenderObjectCreateContext, RenderObjectUpdateContext},
    unit::Axis,
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

use crate::intrinsic::render_intrinsic_dimension::RenderIntrinsicDimension;

/// See [`IntrinsicAxis`] for more information.
#[derive(RenderObjectWidget, Debug)]
pub struct IntrinsicHeight {
    #[prop(into)]
    pub child: Option<Widget>,
}

impl RenderObjectWidget for IntrinsicHeight {
    type RenderObject = RenderIntrinsicDimension;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderIntrinsicDimension {
            axis: Axis::Vertical,
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_axis(ctx, Axis::Vertical);
    }
}
