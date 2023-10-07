use rustc_hash::FxHashSet;

use crate::{
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    util::tree::Tree,
    widget::ContextWidgetStateMut,
};

pub struct StatefulCallbackContext<'ctx, S> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,

    pub(crate) state: &'ctx mut S,
    pub(crate) is_changed: bool,
}

impl<S> ContextElement for StatefulCallbackContext<'_, S> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<S> ContextMarkDirty for StatefulCallbackContext<'_, S> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
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
