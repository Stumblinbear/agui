use std::rc::Rc;

use agui_core::{
    element::{ElementBuilder, ElementType},
    render::binding::RenderBinding,
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use super::element::ViewElement;

#[derive(WidgetProps)]
pub struct View {
    pub binding: Rc<dyn RenderBinding>,

    pub child: Widget,
}

impl IntoWidget for View {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for View {
    fn create_element(self: Rc<Self>) -> ElementType
    where
        Self: Sized,
    {
        ElementType::Render(Box::new(ViewElement::new(self)))
    }
}
