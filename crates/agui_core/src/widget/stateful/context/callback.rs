use fnv::FnvHashSet;

use crate::{
    element::{Element, ElementId},
    inheritance::InheritanceManager,
    util::tree::Tree,
    widget::{ContextWidget, ContextWidgetStateMut},
};

pub struct StatefulCallbackContext<'ctx, S> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance_manager: &'ctx InheritanceManager,

    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) element_id: ElementId,

    pub(crate) state: &'ctx mut S,

    pub(crate) is_changed: bool,
}

impl<S> StatefulCallbackContext<'_, S> {
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl<S> ContextWidget for StatefulCallbackContext<'_, S> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx, S> ContextWidgetStateMut<'ctx, S> for StatefulCallbackContext<'ctx, S> {
    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S) + 'static,
    {
        func(self.state);

        self.is_changed = true;
    }
}
