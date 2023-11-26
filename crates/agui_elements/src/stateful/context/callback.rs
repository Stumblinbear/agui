use agui_core::element::{ContextElements, ElementCallbackContext};

use agui_core::{
    element::{ContextDirtyElement, ContextElement, Element, ElementId},
    util::tree::Tree,
};

use super::ContextWidgetStateMut;

pub struct StatefulCallbackContext<'ctx, 'element, S> {
    pub(crate) inner: &'element mut ElementCallbackContext<'ctx>,

    pub state: &'element mut S,
    pub(crate) is_changed: bool,
}

impl<S> ContextElements for StatefulCallbackContext<'_, '_, S> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl<S> ContextElement for StatefulCallbackContext<'_, '_, S> {
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<S> ContextDirtyElement for StatefulCallbackContext<'_, '_, S> {
    fn mark_needs_build(&mut self) {
        self.inner.mark_needs_build();
    }
}

impl<'ctx, S> ContextWidgetStateMut<'ctx, S> for StatefulCallbackContext<'ctx, '_, S> {
    fn set_state<F>(&mut self, func: F)
    where
        F: FnOnce(&mut S) + 'static,
    {
        func(self.state);

        self.is_changed = true;
    }
}
