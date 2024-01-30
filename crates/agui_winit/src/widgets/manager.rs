use agui_core::{callback::Callback, widget::Widget};
use agui_elements::inherited::InheritedWidget;
use agui_macros::InheritedWidget;
use agui_renderer::BindRenderer;
use winit::window::WindowBuilder;

use crate::{
    app::WinitCreateWindowError,
    controller::{WinitController, WinitEventLoopClosed},
    handle::WindowHandle,
};

#[derive(InheritedWidget)]
pub struct WinitWindowManager {
    pub controller: WinitController,

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
        renderer: impl BindRenderer<winit::window::Window> + Send + 'static,
        callback: Callback<Result<WindowHandle, WinitCreateWindowError>>,
    ) -> Result<(), WinitEventLoopClosed> {
        self.controller
            .create_window(window_fn, renderer, move |result| callback.call(result))
    }

    pub fn shutdown(&self) -> Result<(), WinitEventLoopClosed> {
        self.controller.shutdown()
    }
}
