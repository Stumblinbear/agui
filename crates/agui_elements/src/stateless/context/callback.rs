use agui_core::{
    element::{
        ContextElement, ContextElements, ContextMarkDirty, Element, ElementCallbackContext,
        ElementId,
    },
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
};

pub struct StatelessCallbackContext<'ctx> {
    pub(crate) inner: ElementCallbackContext<'ctx>,
}

impl<'ctx> ContextPlugins<'ctx> for StatelessCallbackContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.inner.plugins()
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for StatelessCallbackContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.inner.plugins_mut()
    }
}

impl ContextElements for StatelessCallbackContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl ContextElement for StatelessCallbackContext<'_> {
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl ContextMarkDirty for StatelessCallbackContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.inner.mark_dirty(element_id);
    }
}
