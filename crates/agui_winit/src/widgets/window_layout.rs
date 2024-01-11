use agui_core::{
    element::{ContextDirtyRenderObject, RenderObjectCreateContext, RenderObjectUpdateContext},
    render::object::{
        RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext,
    },
    unit::{Constraints, IntrinsicDimension, Size},
    widget::Widget,
};
use agui_elements::render::RenderObjectWidget;
use agui_macros::RenderObjectWidget;

#[derive(RenderObjectWidget)]
pub struct WinitWindowLayout {
    size: Size,

    child: Widget,
}

impl RenderObjectWidget for WinitWindowLayout {
    type RenderObject = RenderWinitWindowLayout;

    fn children(&self) -> Vec<Widget> {
        vec![self.child.clone()]
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> Self::RenderObject {
        RenderWinitWindowLayout { size: self.size }
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut Self::RenderObject,
    ) {
        render_object.update_size(ctx, self.size);
    }
}

pub struct RenderWinitWindowLayout {
    size: Size,
}

impl RenderWinitWindowLayout {
    fn update_size(&mut self, ctx: &mut RenderObjectUpdateContext, size: Size) {
        if self.size == size {
            return;
        }

        self.size = size;
        ctx.mark_needs_layout();
    }
}

impl RenderObjectImpl for RenderWinitWindowLayout {
    fn is_sized_by_parent(&self) -> bool {
        true
    }

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

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, _: Constraints) -> Size {
        let mut children = ctx.iter_children_mut();

        while let Some(mut child) = children.next() {
            child.layout(Constraints::from(self.size));
        }

        self.size
    }
}
