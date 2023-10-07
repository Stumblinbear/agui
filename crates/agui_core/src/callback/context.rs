use rustc_hash::FxHashSet;

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::{ContextMarkDirty, ContextWidget},
};

pub struct CallbackContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) element_id: ElementId,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
}

impl ContextWidget for CallbackContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextMarkDirty for CallbackContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
