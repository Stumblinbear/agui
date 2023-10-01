use agui_core::{
    unit::{Axis, Constraints, IntrinsicDimension, Size},
    widget::{IntrinsicSizeContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;

mod intrinsic_height;
mod intrinsic_width;

pub use intrinsic_height::*;
pub use intrinsic_width::*;

/// A widget that sizes its child to the child's maximum intrinsic size along the
/// given axis.
///
/// This is relatively expensive because it adds a speculative layout pass before
/// the final layout phase. Avoid using it where possible. In the worst case, this
/// can result in a layout that is O(N²) in the depth of the tree.
#[derive(LayoutWidget, Debug)]
pub struct IntrinsicAxis {
    pub axis: Axis,

    #[prop(into)]
    pub child: Option<Widget>,
}

impl WidgetLayout for IntrinsicAxis {
    fn get_children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    fn layout(&self, ctx: &mut LayoutContext, mut constraints: Constraints) -> Size {
        if let Some(mut child) = ctx.iter_children_mut().next() {
            if !constraints.has_tight_axis(self.axis) {
                let extent = child.compute_intrinsic_size(
                    IntrinsicDimension::max_axis(self.axis),
                    constraints.max_axis(self.axis.flip()),
                );

                assert!(
                    extent.is_finite(),
                    "IntrinsicAxis must have a child that has a finite maximum intrinsic size along its {:?} axis.",
                    self.axis
                );

                constraints = constraints.tighten_axis(self.axis, extent);
            } else {
                // Technically IntrinsicAxis isn't necessary if the child has a tight axis.
                // Do we want to log anything here? It's not an error, but it could be good
                // to know if this is happening.
            }

            child.compute_layout(constraints)
        } else {
            constraints.smallest()
        }
    }
}
