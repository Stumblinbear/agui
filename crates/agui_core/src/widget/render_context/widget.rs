use crate::widget::{element::WidgetElement, ElementBuilder, IntoWidget, Widget, WidgetChild};

use super::instance::RenderContextBoundaryElement;

#[derive(Default)]
pub struct RenderContextBoundary {
    pub child: Option<Widget>,
}

impl WidgetChild for RenderContextBoundary {
    type Child = Option<Widget>;

    fn get_child(&self) -> Self::Child {
        self.child.clone()
    }
}

impl IntoWidget for RenderContextBoundary {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl From<RenderContextBoundary> for Option<Widget> {
    fn from(val: RenderContextBoundary) -> Self {
        Some(val.into_widget())
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
