use std::{any::Any, ops::Deref, rc::Rc, sync::mpsc};

use agui_sync::notify;

use crate::callback::CallbackId;

#[derive(Clone)]
pub struct CallbackQueue(Rc<LocalCallbackQueue>);

impl CallbackQueue {
    pub(crate) fn new(tx: mpsc::Sender<InvokeCallback>, notifier: notify::Flag) -> Self {
        Self(Rc::new(LocalCallbackQueue { tx, notifier }))
    }
}

pub struct LocalCallbackQueue {
    tx: mpsc::Sender<InvokeCallback>,
    notifier: notify::Flag,
}

impl LocalCallbackQueue {
    /// # Panics
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any>) {
        let _ = self.tx.send(InvokeCallback { callback_id, arg });
        self.notifier.notify();
    }

    pub fn shared(&self) -> SharedCallbackQueue {
        SharedCallbackQueue {
            tx: self.tx.clone(),
            notifier: self.notifier.clone(),
        }
    }
}

impl Deref for CallbackQueue {
    type Target = Rc<LocalCallbackQueue>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct SharedCallbackQueue {
    tx: mpsc::Sender<InvokeCallback>,
    notifier: notify::Flag,
}

impl SharedCallbackQueue {
    /// # Panics
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub fn call_unchecked(
        &self,
        callback_id: CallbackId,
        arg: Box<dyn Any + Send>,
    ) -> Result<(), CallError> {
        if let Err(err) = self.tx.send(InvokeCallback { callback_id, arg }) {
            return Err(CallError(err.0.arg));
        }

        self.notifier.notify();

        Ok(())
    }
}

pub(crate) struct InvokeCallback {
    pub callback_id: CallbackId,
    pub arg: Box<dyn Any>,
}

pub struct CallError(pub Box<dyn Any>);

impl std::fmt::Debug for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CallError").field(&"...").finish()
    }
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("callback channel was closed")
    }
}
