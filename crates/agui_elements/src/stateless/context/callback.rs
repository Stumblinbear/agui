use agui_core::{
    element::{
        ContextDirtyElement, ContextElement, ContextElements, Element, ElementCallbackContext,
        ElementId,
    },
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
};

pub struct StatelessCallbackContext<'ctx, 'element> {
    pub(crate) inner: &'element mut ElementCallbackContext<'ctx>,
}

impl<'ctx> ContextPlugins<'ctx> for StatelessCallbackContext<'ctx, '_> {
    fn plugins(&self) -> &Plugins {
        self.inner.plugins()
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for StatelessCallbackContext<'ctx, '_> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.inner.plugins_mut()
    }
}

impl ContextElements for StatelessCallbackContext<'_, '_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }
}

impl ContextElement for StatelessCallbackContext<'_, '_> {
    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl ContextDirtyElement for StatelessCallbackContext<'_, '_> {
    fn mark_needs_build(&mut self) {
        self.inner.mark_needs_build();
    }
}
