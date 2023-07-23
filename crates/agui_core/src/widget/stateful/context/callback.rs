use fnv::FnvHashSet;

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::{ContextWidget, ContextWidgetState, ContextWidgetStateMut},
};

pub struct StatefulCallbackContext<'ctx, S> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) element_id: ElementId,

    pub(crate) state: &'ctx S,

    pub(crate) set_states: &'ctx mut Vec<Box<dyn FnOnce(&mut S)>>,
}

impl<S> StatefulCallbackContext<'_, S> {
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl<S> ContextWidget<S> for StatefulCallbackContext<'_, S> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<S> ContextWidgetState<S> for StatefulCallbackContext<'_, S> {
    fn get_state(&self) -> &S {
        self.state
    }
}

impl<'ctx, S> ContextWidgetStateMut<'ctx, S> for StatefulCallbackContext<'ctx, S> {
    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S) + 'static,
    {
        self.set_states.push(Box::new(func));
    }
}
