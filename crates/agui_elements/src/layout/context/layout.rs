use std::ops::{Deref, DerefMut};

use agui_core::{
    element::{ContextElement, Element, ElementId, RenderObjectLayoutContext},
    util::tree::Tree,
};

pub struct LayoutContext<'ctx> {
    pub(crate) inner: RenderObjectLayoutContext<'ctx>,
}

impl ContextElement for LayoutContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }

    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<'ctx> Deref for LayoutContext<'ctx> {
    type Target = RenderObjectLayoutContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for LayoutContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
