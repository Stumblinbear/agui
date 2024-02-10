use std::hash::BuildHasherDefault;

use agui_core::{
    element::{deferred::resolver::DeferredResolver, ElementId},
    engine::rendering::RenderingTree,
    render::RenderObjectId,
};
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

// TODO: this can almost certainly be more efficient
pub struct ResolveDeferredElement {
    pub rendering_tree: RenderingTree,

    pub deferred_elements: SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,

    pub render_object_id: RenderObjectId,

    pub reply_tx: oneshot::Sender<ResolveDeferredElementReply>,
}

pub struct ResolveDeferredElementReply {
    pub rendering_tree: RenderingTree,

    pub deferred_elements: SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,

    pub needs_paint: FxHashSet<RenderObjectId>,
}
