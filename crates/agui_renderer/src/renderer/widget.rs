use std::{ops::Deref, sync::Arc};

use agui_core::{util::ptr_eq::PtrEqual, widget::Widget};
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;

use crate::Renderer;

#[derive(InheritedWidget)]
pub struct DefaultRenderer<T: 'static> {
    pub renderer: Arc<dyn Renderer<Target = T>>,

    child: Widget,
}

impl<T> InheritedWidget for DefaultRenderer<T> {
    fn child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, old_widget: &Self) -> bool {
        !self.renderer.is_exact_ptr(&old_widget.renderer)
    }
}

impl<T> Deref for DefaultRenderer<T> {
    type Target = Arc<dyn Renderer<Target = T>>;

    fn deref(&self) -> &Self::Target {
        &self.renderer
    }
}
