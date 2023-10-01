use std::ops::{Deref, DerefMut};

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::{element::WidgetLayoutContext, ContextWidget},
};

pub struct LayoutContext<'ctx> {
    pub(crate) widget_ctx: WidgetLayoutContext<'ctx>,
}

impl ContextWidget for LayoutContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.widget_ctx.get_elements()
    }

    fn get_element_id(&self) -> ElementId {
        self.widget_ctx.get_element_id()
    }
}

impl<'ctx> Deref for LayoutContext<'ctx> {
    type Target = WidgetLayoutContext<'ctx>;

    fn deref(&self) -> &Self::Target {
        &self.widget_ctx
    }
}

impl<'ctx> DerefMut for LayoutContext<'ctx> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget_ctx
    }
}
