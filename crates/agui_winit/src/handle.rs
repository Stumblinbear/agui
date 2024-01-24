use std::{ops::Deref, sync::Arc};

use agui_sync::broadcast;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::WinitWindowEvent;

#[derive(Clone)]
pub struct WinitWindowHandle {
    inner: Arc<winit::window::Window>,

    events_tx: broadcast::UnboundedSender<WinitWindowEvent>,
}

impl WinitWindowHandle {
    pub(crate) fn new(
        handle: winit::window::Window,
        events_tx: broadcast::UnboundedSender<WinitWindowEvent>,
    ) -> Self {
        Self {
            inner: Arc::new(handle),

            events_tx,
        }
    }

    pub async fn subscribe(&self) -> broadcast::UnboundedReceiver<WinitWindowEvent> {
        self.events_tx.subscribe().await
    }
}

impl Deref for WinitWindowHandle {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

unsafe impl HasRawWindowHandle for WinitWindowHandle {
    fn raw_window_handle(&self) -> raw_window_handle::RawWindowHandle {
        self.inner.raw_window_handle()
    }
}

unsafe impl HasRawDisplayHandle for WinitWindowHandle {
    fn raw_display_handle(&self) -> raw_window_handle::RawDisplayHandle {
        self.inner.raw_display_handle()
    }
}
