use std::{error::Error, sync::mpsc};

use agui_core::callback::Callback;
use winit::{
    event_loop::EventLoopProxy,
    window::{WindowBuilder, WindowId},
};

use crate::{app::WinitBindingAction, WinitWindowHandle};

pub struct WinitWindowController {
    event_loop: EventLoopProxy<WinitBindingAction>,
}

impl WinitWindowController {
    pub fn new(event_loop: EventLoopProxy<WinitBindingAction>) -> Self {
        Self { event_loop }
    }

    pub(crate) fn create_window(
        &self,
        window_fn: impl FnOnce() -> WindowBuilder + Send + 'static,
        callback: Callback<WinitWindowHandle>,
    ) -> Result<(), WinitEventLoopClosed> {
        tracing::debug!("queueing window for creation");

        self.event_loop
            .send_event(WinitBindingAction::CreateWindow(
                Box::new(window_fn),
                callback,
            ))?;

        Ok(())
    }

    pub fn render(&self, window_id: WindowId) {
        // if let Some(view_renderer) = self.window_renderer.get(&window_id) {
        //     view_renderer.render();
        // } else {
        //     tracing::error!(
        //         "cannot render to {:?} because no renderer is bound",
        //         window_id
        //     );
        // }
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

impl<T> From<mpsc::SendError<T>> for WinitEventLoopClosed {
    fn from(_: mpsc::SendError<T>) -> Self {
        Self { __private: () }
    }
}

impl<T> From<winit::event_loop::EventLoopClosed<T>> for WinitEventLoopClosed {
    fn from(_: winit::event_loop::EventLoopClosed<T>) -> Self {
        Self { __private: () }
    }
}
