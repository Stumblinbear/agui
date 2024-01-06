use agui_core::{callback::Callback, widget::Widget};
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;
use winit::window::WindowBuilder;

use crate::{controller::WinitEventLoopClosed, WinitWindowController, WinitWindowHandle};

#[derive(InheritedWidget)]
pub struct WinitWindowManager {
    pub controller: WinitWindowController,

    pub child: Widget,
}

impl InheritedWidget for WinitWindowManager {
    fn child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}

impl WinitWindowManager {
    pub fn create_window(
        &self,
        window_fn: impl FnOnce() -> WindowBuilder + Send + 'static,
        callback: Callback<WinitWindowHandle>,
    ) -> Result<(), WinitEventLoopClosed> {
        self.controller.create_window(window_fn, callback)
    }
}
