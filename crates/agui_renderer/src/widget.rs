use std::rc::Rc;

use agui_core::{
    element::{ElementBuilder, ElementType},
    widget::{IntoWidget, Widget},
};

use crate::element::RenderViewElement;

pub struct RenderView {
    pub child: Widget,
}

impl IntoWidget for RenderView {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for RenderView {
    fn create_element(self: Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::Proxy(Box::new(RenderViewElement::new(self)))
    }
}
