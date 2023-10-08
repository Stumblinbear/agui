use crate::{
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    engine::DirtyElements,
    util::tree::Tree,
};

pub struct PluginUnmountContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub dirty: &'ctx mut DirtyElements,

    pub element_id: &'ctx ElementId,
}

impl ContextElement for PluginUnmountContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextMarkDirty for PluginUnmountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
