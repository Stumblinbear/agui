use agui_core::{
    element::{deferred::resolver::DeferredResolver, ElementId},
    engine::rendering::strategies::RenderingTreeCleanupStrategy,
    render::RenderObjectId,
};
use slotmap::SecondaryMap;

pub struct RenderingTreeCleanup<'cleanup> {
    pub deferred_elements:
        &'cleanup mut SecondaryMap<RenderObjectId, (ElementId, Box<dyn DeferredResolver>)>,
}

impl RenderingTreeCleanupStrategy for RenderingTreeCleanup<'_> {
    fn on_removed(&mut self, render_object_id: RenderObjectId) {
        tracing::trace!(?render_object_id, "removed render object");

        self.deferred_elements.remove(render_object_id);
    }
}
