use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    util::tree::Tree,
};

pub struct PluginBuildContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,

    pub(crate) callback_queue: &'ctx CallbackQueue,
}

impl ContextElement for PluginBuildContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextMarkDirty for PluginBuildContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
