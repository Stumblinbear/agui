use crate::{
    element::ElementType,
    widget::{ElementBuilder, IntoWidget, Widget},
};

use super::instance::RenderViewElement;

pub struct RenderView {
    pub child: Widget,
}

impl IntoWidget for RenderView {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for RenderView {
    fn create_element(self: std::rc::Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::View(Box::new(RenderViewElement::new(self)))
    }
}
