use agui_core::{
    element::{RenderObjectBuildContext, RenderObjectUpdateContext},
    unit::Axis,
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

mod intrinsic_height;
mod intrinsic_width;
mod render_intrinsic_dimension;

pub use intrinsic_height::*;
pub use intrinsic_width::*;

use crate::intrinsic::render_intrinsic_dimension::RenderIntrinsicDimension;

/// A widget that sizes its child to the child's maximum intrinsic size along the
/// given axis.
///
/// This is relatively expensive because it adds a speculative layout pass before
/// the final layout phase. Avoid using it where possible. In the worst case, this
/// can result in a layout that is O(N²) in the depth of the tree.
#[derive(RenderObjectWidget, Debug)]
pub struct IntrinsicAxis {
    pub axis: Axis,

    #[prop(into)]
    pub child: Option<Widget>,
}

impl RenderObjectWidget for IntrinsicAxis {
    type RenderObject = RenderIntrinsicDimension;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectBuildContext) -> Self::RenderObject {
        RenderIntrinsicDimension { axis: self.axis }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_axis(ctx, self.axis);
    }
}
