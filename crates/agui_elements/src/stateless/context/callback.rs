use std::ops::{Deref, DerefMut};

use agui_core::{
    element::{ContextElement, ContextElements, Element, ElementCallbackContext, ElementId},
    util::tree::Tree,
};

pub struct StatelessCallbackContext<'ctx, 'element> {
    pub(crate) inner: &'element mut ElementCallbackContext<'ctx>,
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

impl<'ctx> Deref for StatelessCallbackContext<'ctx, '_> {
    type Target = ElementCallbackContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ctx> DerefMut for StatelessCallbackContext<'ctx, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}
