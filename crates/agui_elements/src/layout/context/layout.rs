use std::ops::{Deref, DerefMut};

use agui_core::{
    element::{ContextElement, Element, ElementId, ElementLayoutContext},
    util::tree::Tree,
};

pub struct LayoutContext<'ctx> {
    pub(crate) inner: ElementLayoutContext<'ctx>,
}

impl ContextElement for LayoutContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.inner.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.inner.get_element_id()
    }
}

impl<'ctx> Deref for LayoutContext<'ctx> {
    type Target = ElementLayoutContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for LayoutContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
