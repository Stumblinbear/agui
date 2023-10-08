use std::ops::{Deref, DerefMut};

use agui_core::{
    element::{ContextElement, Element, ElementId, ElementIntrinsicSizeContext},
    util::tree::Tree,
};

pub struct IntrinsicSizeContext<'ctx> {
    pub(crate) inner: ElementIntrinsicSizeContext<'ctx>,
}

impl ContextElement for IntrinsicSizeContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.inner.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.inner.get_element_id()
    }
}

impl<'ctx> Deref for IntrinsicSizeContext<'ctx> {
    type Target = ElementIntrinsicSizeContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for IntrinsicSizeContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
