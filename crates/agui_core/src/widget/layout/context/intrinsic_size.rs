use std::ops::{Deref, DerefMut};

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::{element::WidgetIntrinsicSizeContext, ContextElement},
};

pub struct IntrinsicSizeContext<'ctx> {
    pub(crate) widget_ctx: WidgetIntrinsicSizeContext<'ctx>,
}

impl ContextElement for IntrinsicSizeContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx> Deref for IntrinsicSizeContext<'ctx> {
    type Target = WidgetIntrinsicSizeContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.widget_ctx
    }
}

impl DerefMut for IntrinsicSizeContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget_ctx
    }
}
