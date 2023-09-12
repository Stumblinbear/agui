use crate::widget::{element::WidgetElement, ElementBuilder, IntoWidget, Widget, WidgetChild};

use super::instance::RenderContextBoundaryElement;

pub struct RenderContextBoundary {
    pub child: Widget,
}

impl RenderContextBoundary {
    pub fn with_child(child: impl IntoWidget) -> Self {
        Self {
            child: child.into_widget(),
        }
    }
}

impl WidgetChild for RenderContextBoundary {
    fn get_child(&self) -> Widget {
        self.child.clone()
    }
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
