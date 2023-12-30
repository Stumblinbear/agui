use std::{
    ops::Deref,
    sync::{mpsc, Arc},
};

use agui_core::listenable::EventEmitter;

use crate::{
    controller::{WinitBindingAction, WinitSendError},
    WinitWindowEvent,
};

#[derive(Clone)]
pub struct WinitWindowHandle {
    pub(crate) handle: Arc<winit::window::Window>,
    pub(crate) event_emitter: EventEmitter<WinitWindowEvent>,
    pub(crate) action_queue_tx: mpsc::Sender<WinitBindingAction>,
}

impl WinitWindowHandle {
    pub fn events(&self) -> &EventEmitter<WinitWindowEvent> {
        &self.event_emitter
    }

    pub(crate) fn close(&self) -> Result<(), WinitSendError> {
        Ok(self
            .action_queue_tx
            .send(WinitBindingAction::CloseWindow(self.handle.id()))?)
    }
}

impl Deref for WinitWindowHandle {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        self.handle.as_ref()
    }
}
