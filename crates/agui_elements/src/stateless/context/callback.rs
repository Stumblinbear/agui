use agui_core::{
    element::{ContextElement, ContextMarkDirty, Element, ElementCallbackContext, ElementId},
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
    fn get_plugins(&self) -> &Plugins {
        self.inner.get_plugins()
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for StatelessCallbackContext<'ctx> {
    fn get_plugins_mut(&mut self) -> &mut Plugins {
        self.inner.get_plugins_mut()
    }
}

impl ContextElement for StatelessCallbackContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.inner.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.inner.get_element_id()
    }
}

impl ContextMarkDirty for StatelessCallbackContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.inner.mark_dirty(element_id);
    }
}
