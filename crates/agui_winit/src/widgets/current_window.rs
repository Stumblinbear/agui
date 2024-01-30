use std::ops::Deref;

use agui_core::widget::Widget;
use agui_elements::inherited::InheritedWidget;
use agui_macros::InheritedWidget;

use crate::handle::WindowHandle;

#[derive(InheritedWidget)]
pub struct CurrentWindow {
    handle: WindowHandle,

    child: Widget,
}

impl Deref for CurrentWindow {
    type Target = WindowHandle;

    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl InheritedWidget for CurrentWindow {
    fn child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, old_widget: &Self) -> bool {
        self.handle.id() != old_widget.handle.id()
    }
}
