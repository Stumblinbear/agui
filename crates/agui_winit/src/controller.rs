use std::{
    error::Error,
    sync::{mpsc, Arc},
};

use agui_core::callback::Callback;
use agui_renderer::RenderManifold;
use rustc_hash::FxHashMap;
use winit::window::{WindowBuilder, WindowId};

use crate::WinitWindowHandle;

pub struct WinitWindowController {
    windows: FxHashMap<WindowId, WinitWindowHandle>,
    window_renderer: FxHashMap<WindowId, Arc<dyn RenderManifold>>,

    event_notifier_tx: mpsc::Sender<()>,

    action_queue_tx: mpsc::Sender<WinitBindingAction>,
    action_queue_rx: mpsc::Receiver<WinitBindingAction>,
}

impl WinitWindowController {
    pub fn new(event_notifier_tx: mpsc::Sender<()>) -> Self {
        let (action_queue_tx, action_queue_rx) = mpsc::channel();

        Self {
            windows: FxHashMap::default(),
            window_renderer: FxHashMap::default(),

            event_notifier_tx,

            action_queue_tx,
            action_queue_rx,
        }
    }

    pub fn get_window(&self, window_id: WindowId) -> Option<&WinitWindowHandle> {
        self.windows.get(&window_id)
    }

    pub(crate) fn create_window(
        &self,
        window: WindowBuilder,
        callback: Callback<WinitWindowHandle>,
    ) -> Result<(), WinitSendError> {
        tracing::debug!("queueing window for creation");

        self.action_queue_tx
            .send(WinitBindingAction::CreateWindow(Box::new(window), callback))?;

        self.event_notifier_tx.send(())?;

        Ok(())
    }

    pub fn render(&self, window_id: WindowId) {
        if let Some(view_renderer) = self.window_renderer.get(&window_id) {
            view_renderer.render();
        } else {
            tracing::error!(
                "cannot render to {:?} because no renderer is bound",
                window_id
            );
        }
    }
}

pub enum WinitBindingAction {
    CreateWindow(Box<WindowBuilder>, Callback<WinitWindowHandle>),
    CloseWindow(WindowId),
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct WinitSendError {
    __private: (),
}

impl std::fmt::Debug for WinitSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinitSendError").finish()
    }
}

impl std::fmt::Display for WinitSendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        "sending on a closed channel".fmt(f)
    }
}

impl Error for WinitSendError {}

impl<T> From<mpsc::SendError<T>> for WinitSendError {
    fn from(_: mpsc::SendError<T>) -> Self {
        Self { __private: () }
    }
}
