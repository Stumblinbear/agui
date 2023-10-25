use std::ops::{Deref, DerefMut};

use agui_core::{
    element::{ContextElement, Element, ElementId, RenderObjectHitTestContext},
    util::tree::Tree,
};

pub struct HitTestContext<'ctx> {
    pub(crate) inner: &'ctx mut RenderObjectHitTestContext<'ctx>,
}

impl ContextElement for HitTestContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.inner.elements()
    }

    fn element_id(&self) -> ElementId {
        self.inner.element_id()
    }
}

impl<'ctx> Deref for HitTestContext<'ctx> {
    type Target = RenderObjectHitTestContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl DerefMut for HitTestContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}
