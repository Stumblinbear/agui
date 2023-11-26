use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectUpdateContext},
    render::{RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext},
    unit::{Alignment, Constraints, IntrinsicDimension, Size},
};

#[derive(Debug)]
pub struct RenderAlignedBox {
    pub alignment: Alignment,

    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
}

impl RenderAlignedBox {
    pub fn update_alignment(&mut self, ctx: &mut RenderObjectUpdateContext, alignment: Alignment) {
        self.alignment = alignment;
        ctx.mark_needs_layout();
    }

    pub fn update_width_factor(
        &mut self,
        ctx: &mut RenderObjectUpdateContext,
        width_factor: Option<f32>,
    ) {
        self.width_factor = width_factor;
        ctx.mark_needs_layout();
    }

    pub fn update_height_factor(
        &mut self,
        ctx: &mut RenderObjectUpdateContext,
        height_factor: Option<f32>,
    ) {
        self.height_factor = height_factor;
        ctx.mark_needs_layout();
    }
}

impl RenderObjectImpl for RenderAlignedBox {
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

    fn layout(&mut self, ctx: &mut RenderObjectLayoutContext, constraints: Constraints) -> Size {
        let shrink_wrap_width = self.width_factor.is_some() || !constraints.has_bounded_width();
        let shrink_wrap_height = self.height_factor.is_some() || !constraints.has_bounded_height();

        if let Some(mut child) = ctx.iter_children_mut().next() {
            let child_size = child.compute_layout(constraints.loosen());

            let size = constraints.constrain(Size {
                width: shrink_wrap_width
                    .then(|| child_size.width * self.width_factor.unwrap_or(1.0))
                    .unwrap_or(f32::INFINITY),

                height: shrink_wrap_height
                    .then(|| child_size.height * self.height_factor.unwrap_or(1.0))
                    .unwrap_or(f32::INFINITY),
            });

            child.set_offset(self.alignment.along_size(size - child_size));

            size
        } else {
            constraints.constrain(Size {
                width: if shrink_wrap_width {
                    0.0
                } else {
                    f32::INFINITY
                },

                height: if shrink_wrap_height {
                    0.0
                } else {
                    f32::INFINITY
                },
            })
        }
    }
}
