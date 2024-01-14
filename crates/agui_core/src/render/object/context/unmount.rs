use crate::{element::ContextRenderObject, render::RenderObjectId};

pub struct RenderObjectUnmountContext<'ctx> {
    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextRenderObject for RenderObjectUnmountContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}
