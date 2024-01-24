use std::error::Error;

use agui_renderer::BindRenderer;
use winit::{event_loop::EventLoopProxy, window::WindowBuilder};

use crate::{
    app::{WinitBindingAction, WinitCreateWindowError},
    WinitWindowHandle,
};

#[derive(Clone)]
pub struct WinitController {
    event_loop: EventLoopProxy<WinitBindingAction>,
}

impl WinitController {
    pub(crate) fn new(event_loop: EventLoopProxy<WinitBindingAction>) -> Self {
        Self { event_loop }
    }

    pub fn create_window(
        &self,
        window_fn: impl FnOnce() -> WindowBuilder + Send + 'static,
        renderer: impl BindRenderer<winit::window::Window> + Send + 'static,
        callback: impl FnOnce(Result<WinitWindowHandle, WinitCreateWindowError>) + Send + 'static,
    ) -> Result<(), WinitEventLoopClosed> {
        tracing::debug!("queueing window for creation");

        self.event_loop
            .send_event(WinitBindingAction::CreateWindow(
                Box::new(window_fn),
                Box::new(move |window, frame_notifier| {
                    Box::new(renderer.bind(window, frame_notifier))
                }),
                Box::new(callback),
            ))?;

        Ok(())
    }

    pub fn shutdown(&self) -> Result<(), WinitEventLoopClosed> {
        self.event_loop.send_event(WinitBindingAction::Shutdown)?;
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
