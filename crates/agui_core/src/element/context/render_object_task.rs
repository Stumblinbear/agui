use crate::{
    element::{ContextDirtyRenderObject, ContextRenderObject, ElementId},
    engine::Dirty,
    render::RenderObjectId,
};

use super::ContextElement;

pub struct RenderObjectTaskContext {
    pub(crate) element_id: ElementId,
    pub(crate) render_object_id: RenderObjectId,

    pub(crate) needs_layout: Dirty<RenderObjectId>,
    pub(crate) needs_paint: Dirty<RenderObjectId>,
}

impl ContextElement for RenderObjectTaskContext {
    fn element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextRenderObject for RenderObjectTaskContext {
    fn render_object_id(&self) -> RenderObjectId {
        self.render_object_id
    }
}

impl ContextDirtyRenderObject for RenderObjectTaskContext {
    fn mark_needs_layout(&mut self) {
        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs layout");

        self.needs_layout.insert(self.render_object_id);
        self.needs_layout.notify();
    }

    fn mark_needs_paint(&mut self) {
        tracing::trace!(render_object_id = ?self.render_object_id, "render object needs paint");

        self.needs_paint.insert(self.render_object_id);
        self.needs_layout.notify();
    }
}
