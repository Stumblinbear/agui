use agui_core::{
    element::{RenderObjectBuildContext, RenderObjectUpdateContext},
    unit::Axis,
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

use crate::intrinsic::render_intrinsic_dimension::RenderIntrinsicDimension;

/// See [`IntrinsicAxis`] for more information.
#[derive(RenderObjectWidget, Debug)]
pub struct IntrinsicWidth {
    #[prop(into)]
    pub child: Option<Widget>,
}

impl RenderObjectWidget for IntrinsicWidth {
    type RenderObject = RenderIntrinsicDimension;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectBuildContext) -> Self::RenderObject {
        RenderIntrinsicDimension {
            axis: Axis::Horizontal,
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_axis(ctx, Axis::Horizontal);
    }
}
