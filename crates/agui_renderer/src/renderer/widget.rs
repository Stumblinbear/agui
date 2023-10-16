use std::{ops::Deref, sync::Arc};

use agui_core::widget::Widget;
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;

use crate::Renderer;

#[derive(InheritedWidget)]
pub struct DefaultRenderer<T: 'static> {
    pub renderer: Arc<dyn Renderer<Target = T>>,

    #[prop(into)]
    child: Widget,
}

impl<T> InheritedWidget for DefaultRenderer<T> {
    fn get_child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, old_widget: &Self) -> bool {
        !std::ptr::eq(
            Arc::as_ptr(&self.renderer) as *const _ as *const (),
            Arc::as_ptr(&old_widget.renderer) as *const _ as *const (),
        )
    }
}

impl<T> Deref for DefaultRenderer<T> {
    type Target = Arc<dyn Renderer<Target = T>>;

    fn deref(&self) -> &Self::Target {
        &self.renderer
    }
}
