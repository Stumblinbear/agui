use agui_core::{
    engine::rendering::strategies::RenderingTreeCleanupStrategy, render::RenderObjectId,
};

pub struct RenderingTreeCleanup<'cleanup> {
    pub removed_deferred_elements: &'cleanup mut Vec<RenderObjectId>,
}

impl RenderingTreeCleanupStrategy for RenderingTreeCleanup<'_> {
    fn on_removed(&mut self, render_object_id: RenderObjectId) {
        tracing::trace!(?render_object_id, "removed render object");

        self.removed_deferred_elements.push(render_object_id);
    }
}
