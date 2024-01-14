use crate::{element::ContextRenderObject, render::RenderObjectId};

pub struct RenderObjectMountContext<'ctx> {
    pub parent_render_object_id: &'ctx Option<RenderObjectId>,
    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextRenderObject for RenderObjectMountContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl RenderObjectMountContext<'_> {
    pub fn parent_render_object_id(&self) -> Option<RenderObjectId> {
        *self.parent_render_object_id
    }
}
