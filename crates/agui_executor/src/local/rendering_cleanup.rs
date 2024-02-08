use std::hash::BuildHasherDefault;

use agui_core::{
    element::{deferred::resolver::DeferredResolver, ElementId},
    engine::rendering::strategies::RenderingTreeCleanupStrategy,
    render::RenderObjectId,
};
use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

pub struct RenderingTreeCleanup<'cleanup> {
    pub deferred_elements: &'cleanup mut SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,
}

impl RenderingTreeCleanupStrategy for RenderingTreeCleanup<'_> {
    fn on_removed(&mut self, render_object_id: RenderObjectId) {
        tracing::trace!(?render_object_id, "removed render object");

        self.deferred_elements.remove(render_object_id);
    }
}
