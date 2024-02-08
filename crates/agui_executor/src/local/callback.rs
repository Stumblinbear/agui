use std::{any::Any, sync::mpsc};

use agui_core::callback::{strategies::CallbackStrategy, CallbackId};
use agui_sync::notify;

pub struct LocalCallbacks {
    pub callback_tx: mpsc::Sender<InvokeCallback>,
    pub element_update_tx: notify::Flag,
}

impl CallbackStrategy for LocalCallbacks {
    fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any + Send>) {
        if let Err(err) = self.callback_tx.send(InvokeCallback { callback_id, arg }) {
            tracing::error!(?err, "failed to send callback");
        } else {
            self.element_update_tx.notify();
        }
    }
}

#[non_exhaustive]
pub struct InvokeCallback {
    pub callback_id: CallbackId,
    pub arg: Box<dyn Any>,
}
