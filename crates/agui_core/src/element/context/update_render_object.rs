use crate::{
    element::{ContextDirtyRenderObject, ElementId},
    render::RenderObjectId,
};

use super::{ContextElement, ContextRenderObject};

pub struct RenderObjectUpdateContext<'ctx> {
    pub needs_layout: &'ctx mut bool,
    pub needs_paint: &'ctx mut bool,

    pub element_id: &'ctx ElementId,
    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextElement for RenderObjectUpdateContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextRenderObject for RenderObjectUpdateContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl ContextDirtyRenderObject for RenderObjectUpdateContext<'_> {
    fn mark_needs_layout(&mut self) {
        *self.needs_layout = true;
    }

    fn mark_needs_paint(&mut self) {
        *self.needs_paint = true;
    }
}
