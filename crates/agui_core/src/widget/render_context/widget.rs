use crate::widget::{element::WidgetElement, ElementBuilder, IntoWidget, Widget};

use super::instance::RenderContextBoundaryElement;

pub struct RenderContextBoundary {
    pub child: Widget,
}

impl IntoWidget for RenderContextBoundary {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for RenderContextBoundary {
    fn create_element(self: std::rc::Rc<Self>) -> Box<dyn WidgetElement>
    where
        Self: Sized,
    {
        Box::new(RenderContextBoundaryElement::new(self))
    }
}
