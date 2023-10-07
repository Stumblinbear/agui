use rustc_hash::FxHashSet;

use crate::{
    element::{Element, ElementId},
    render::manager::RenderViewManager,
    util::tree::Tree,
    widget::{ContextElement, ContextMarkDirty},
};

pub struct PluginUnmountContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) element_id: ElementId,
}

impl ContextElement for PluginUnmountContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextMarkDirty for PluginUnmountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
