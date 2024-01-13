use std::error::Error;

use agui_core::{callback::Callback, widget::Widget};
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;
use agui_renderer::RenderWindow;
use winit::{event_loop::EventLoopProxy, window::WindowBuilder};

use crate::{app::WinitBindingAction, WinitWindowHandle};

#[derive(InheritedWidget)]
pub struct WinitWindowManager {
    event_loop: EventLoopProxy<WinitBindingAction>,

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
    pub fn create_window<Renderer>(
        &self,
        window_fn: impl FnOnce() -> WindowBuilder + Send + 'static,
        renderer_fn: impl FnOnce(WinitWindowHandle) -> Renderer + Send + 'static,
        callback: Callback<WinitWindowHandle>,
    ) -> Result<(), WinitEventLoopClosed>
    where
        Renderer: RenderWindow + 'static,
    {
        tracing::debug!("queueing window for creation");

        self.event_loop
            .send_event(WinitBindingAction::CreateWindow(
                Box::new(window_fn),
                Box::new(move |window_handle| Box::new((renderer_fn)(window_handle))),
                callback,
            ))?;

        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct WinitEventLoopClosed {
    __private: (),
}

impl std::fmt::Debug for WinitEventLoopClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinitEventLoopClosed").finish()
    }
}

impl std::fmt::Display for WinitEventLoopClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "sending on a closed channel".fmt(f)
    }
}

impl Error for WinitEventLoopClosed {}

impl<T> From<winit::event_loop::EventLoopClosed<T>> for WinitEventLoopClosed {
    fn from(_: winit::event_loop::EventLoopClosed<T>) -> Self {
        Self { __private: () }
    }
}
