use agui_core::element::ElementCallbackContext;

use agui_core::{
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    util::tree::Tree,
};

use super::ContextWidgetStateMut;

pub struct StatefulCallbackContext<'ctx, S> {
    pub(crate) inner: ElementCallbackContext<'ctx>,

    pub(crate) state: &'ctx mut S,
    pub(crate) is_changed: bool,
}

impl<S> ContextElement for StatefulCallbackContext<'_, S> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.inner.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.inner.get_element_id()
    }
}

impl<S> ContextMarkDirty for StatefulCallbackContext<'_, S> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.inner.mark_dirty(element_id);
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
