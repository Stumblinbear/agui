use std::ops::{Deref, DerefMut};

use agui_core::{
    element::{ContextElement, Element, ElementId, RenderObjectIntrinsicSizeContext},
    util::tree::Tree,
};

pub struct IntrinsicSizeContext<'ctx> {
    pub(crate) inner: RenderObjectIntrinsicSizeContext<'ctx>,
}

impl ContextElement for IntrinsicSizeContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }

    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<'ctx> Deref for IntrinsicSizeContext<'ctx> {
    type Target = RenderObjectIntrinsicSizeContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for IntrinsicSizeContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
