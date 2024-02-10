use agui_core::{
    element::{deferred::resolver::DeferredResolver, ElementId},
    render::RenderObjectId,
};
use rustc_hash::FxHashSet;

#[derive(Default)]
pub(super) struct SyncRenderingTree {
    #[allow(clippy::type_complexity)]
    pub new_deferred_elements: Vec<(RenderObjectId, (ElementId, Box<dyn DeferredResolver>))>,
    pub removed_deferred_elements: Vec<RenderObjectId>,

    pub needs_layout: FxHashSet<RenderObjectId>,
    pub needs_paint: FxHashSet<RenderObjectId>,
}

impl SyncRenderingTree {
    pub fn is_empty(&self) -> bool {
        self.new_deferred_elements.is_empty()
            && self.removed_deferred_elements.is_empty()
            && self.needs_layout.is_empty()
            && self.needs_paint.is_empty()
    }
}
