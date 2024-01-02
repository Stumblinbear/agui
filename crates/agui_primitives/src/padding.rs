use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectCreateContext, RenderObjectUpdateContext},
    render::{RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext},
    unit::{Constraints, EdgeInsets, IntrinsicDimension, Offset, Size},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

#[derive(RenderObjectWidget, Debug)]
pub struct Padding {
    pub padding: EdgeInsets,

    #[prop(into)]
    pub child: Option<Widget>,
}

impl RenderObjectWidget for Padding {
    type RenderObject = RenderPadding;

    fn children(&self) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderPadding {
            padding: self.padding,
        }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_padding(ctx, self.padding);
    }
}

pub struct RenderPadding {
    pub padding: EdgeInsets,
}

impl RenderPadding {
    pub fn update_padding(&mut self, ctx: &mut RenderObjectUpdateContext, padding: EdgeInsets) {
        if self.padding == padding {
            return;
        }

        self.padding = padding;
        ctx.mark_needs_layout();
    }
}

impl RenderObjectImpl for RenderPadding {
    fn intrinsic_size(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        // TODO: should padding even be included in the intrinsic size?
        self.padding.axis(dimension.axis())
            + ctx
                .iter_children()
                .next()
                .map(|child| child.compute_intrinsic_size(dimension, cross_extent))
                .unwrap_or(0.0)
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, constraints: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.layout(constraints.deflate(self.padding));
            child.set_offset(Offset {
                x: self.padding.left,
                y: self.padding.top,
            })
        }

        constraints.biggest()
    }
}
