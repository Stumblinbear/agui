use crate::{
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    engine::DirtyElements,
    util::tree::Tree,
};

pub struct PluginMountContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub dirty: &'ctx mut DirtyElements,

    pub parent_element_id: Option<&'ctx ElementId>,
    pub element_id: &'ctx ElementId,
}

impl ContextElement for PluginMountContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextMarkDirty for PluginMountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl PluginMountContext<'_> {
    pub fn get_parent_element_id(&self) -> Option<ElementId> {
        self.parent_element_id.copied()
    }
}
