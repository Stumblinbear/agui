use agui_core::render::{RenderObject, RenderObjectId};

pub trait RenderManifold {
    fn on_attach(
        &self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
        render_object: &RenderObject,
    );

    fn on_detach(&self, render_object_id: RenderObjectId);

    fn render(&self);
}
