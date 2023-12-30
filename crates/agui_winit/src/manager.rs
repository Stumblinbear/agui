use std::sync::Arc;

use agui_core::{callback::Callback, util::ptr_eq::PtrEqual, widget::Widget};
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;
use winit::window::WindowBuilder;

use crate::{
    controller::{WinitBindingAction, WinitSendError},
    WinitWindowController, WinitWindowHandle,
};

#[derive(InheritedWidget)]
pub struct WinitWindowManager {
    controller: Arc<WinitWindowController>,

    pub child: Widget,
}

impl InheritedWidget for WinitWindowManager {
    fn child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, old_widget: &Self) -> bool {
        !self.controller.is_exact_ptr(&old_widget.controller)
    }
}

impl WinitWindowManager {
    pub fn create_window(
        &self,
        window: WindowBuilder,
        callback: Callback<WinitWindowHandle>,
    ) -> Result<(), WinitSendError> {
        self.controller.create_window(window, callback)
    }
}
