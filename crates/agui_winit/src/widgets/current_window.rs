use std::ops::Deref;

use agui_core::widget::Widget;
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;

use crate::WinitWindowHandle;

#[derive(InheritedWidget)]
pub struct CurrentWindow {
    handle: WinitWindowHandle,

    child: Widget,
}

impl Deref for CurrentWindow {
    type Target = WinitWindowHandle;

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
