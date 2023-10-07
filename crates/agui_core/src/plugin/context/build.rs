use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    util::tree::Tree,
    widget::ContextMarkDirty,
};

pub struct PluginBuildContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) element_id: ElementId,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) callback_queue: &'ctx CallbackQueue,
}

impl ContextMarkDirty for PluginBuildContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
