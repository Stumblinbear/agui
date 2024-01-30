use std::{
    ops::Deref,
    sync::{Arc, Weak},
};

use agui_sync::broadcast;

use crate::WinitWindowEvent;

#[derive(Clone)]
pub struct WindowHandle {
    inner: Arc<winit::window::Window>,

    events_tx: broadcast::UnboundedSender<WinitWindowEvent>,
}

impl WindowHandle {
    pub(crate) fn new(
        handle: Arc<winit::window::Window>,
        events_tx: broadcast::UnboundedSender<WinitWindowEvent>,
    ) -> Self {
        Self {
            inner: handle,

            events_tx,
        }
    }

    pub async fn subscribe(&self) -> broadcast::UnboundedReceiver<WinitWindowEvent> {
        self.events_tx.subscribe().await
    }

    pub fn downgrade(&self) -> WeakWindowHandle {
        WeakWindowHandle {
            inner: Arc::downgrade(&self.inner),

            events_tx: self.events_tx.clone(),
        }
    }
}

impl Deref for WindowHandle {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl PartialEq for WindowHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

pub struct WeakWindowHandle {
    inner: Weak<winit::window::Window>,

    events_tx: broadcast::UnboundedSender<WinitWindowEvent>,
}

impl WeakWindowHandle {
    pub async fn subscribe(&self) -> broadcast::UnboundedReceiver<WinitWindowEvent> {
        self.events_tx.subscribe().await
    }

    pub fn upgrade(&self) -> Option<WindowHandle> {
        self.inner.upgrade().map(|inner| WindowHandle {
            inner,

            events_tx: self.events_tx.clone(),
        })
    }
}

impl PartialEq for WeakWindowHandle {
    fn eq(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.inner, &other.inner)
    }
}
