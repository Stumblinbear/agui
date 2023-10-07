use rustc_hash::FxHashSet;

use crate::{
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    util::tree::Tree,
};

pub struct PluginMountContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) parent_element_id: Option<ElementId>,
    pub(crate) element_id: ElementId,
}

impl ContextElement for PluginMountContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextMarkDirty for PluginMountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl PluginMountContext<'_> {
    pub fn get_parent_element_id(&self) -> Option<ElementId> {
        self.parent_element_id
    }
}
