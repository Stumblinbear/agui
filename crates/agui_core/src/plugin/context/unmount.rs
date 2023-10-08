use crate::{
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    engine::DirtyElements,
    plugin::Plugins,
    util::tree::Tree,
};

pub struct PluginElementUnmountContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub dirty: &'ctx mut DirtyElements,

    pub element_id: &'ctx ElementId,
}

impl ContextElement for PluginElementUnmountContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextMarkDirty for PluginElementUnmountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
