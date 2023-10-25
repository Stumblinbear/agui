use crate::{
    element::{ContextElement, ContextElements, ContextMarkDirty, Element, ElementId},
    engine::DirtyElements,
    util::tree::Tree,
};

pub struct PluginElementUnmountContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub dirty: &'ctx mut DirtyElements,

    pub element_id: &'ctx ElementId,
    pub element: &'ctx Element,
}

impl ContextElements for PluginElementUnmountContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for PluginElementUnmountContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextMarkDirty for PluginElementUnmountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
