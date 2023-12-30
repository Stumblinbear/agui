use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectUpdateContext},
    render::{RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext},
    unit::{Axis, Constraints, IntrinsicDimension, Size},
};

#[derive(Debug)]
pub struct RenderIntrinsicDimension {
    pub axis: Axis,
}

impl RenderIntrinsicDimension {
    pub fn update_axis(&mut self, ctx: &mut RenderObjectUpdateContext, axis: Axis) {
        if self.axis == axis {
            return;
        }

        self.axis = axis;

        ctx.mark_needs_layout();
    }
}

impl RenderObjectImpl for RenderIntrinsicDimension {
    fn intrinsic_size(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, mut constraints: Constraints) -> Size {
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
