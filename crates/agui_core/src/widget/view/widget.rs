use crate::widget::{element::WidgetElement, ElementBuilder, IntoWidget, Widget};

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
    fn create_element(self: std::rc::Rc<Self>) -> Box<dyn WidgetElement>
    where
        Self: Sized,
    {
        Box::new(RenderViewElement::new(self))
    }
}
