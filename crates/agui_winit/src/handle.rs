use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::WinitWindowEvent;

#[derive(Clone)]
pub struct WinitWindowHandle {
    inner: Arc<winit::window::Window>,

    events: async_channel::Receiver<WinitWindowEvent>,
}

impl WinitWindowHandle {
    pub(crate) fn new(
        handle: winit::window::Window,
        events: async_channel::Receiver<WinitWindowEvent>,
    ) -> Self {
        Self {
            inner: Arc::new(handle),

            events,
        }
    }

    pub fn events(&self) -> async_channel::Receiver<WinitWindowEvent> {
        self.events.clone()
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
