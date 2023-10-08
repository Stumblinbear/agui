use std::ops::{Deref, DerefMut};

use agui_core::{
    element::{ContextElement, Element, ElementHitTestContext, ElementId},
    util::tree::Tree,
};

pub struct HitTestContext<'ctx> {
    pub(crate) inner: &'ctx mut ElementHitTestContext<'ctx>,
}

impl ContextElement for HitTestContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.inner.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.inner.get_element_id()
    }
}

impl<'ctx> Deref for HitTestContext<'ctx> {
    type Target = ElementHitTestContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl DerefMut for HitTestContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}
