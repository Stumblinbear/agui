use std::{any::Any, ops::Deref, rc::Rc, sync::mpsc};

use crate::engine::update_notifier::UpdateNotifier;

use super::CallbackId;

#[derive(Clone)]
pub struct CallbackQueue(Rc<LocalCallbackQueue>);

impl CallbackQueue {
    pub(crate) fn new(tx: mpsc::Sender<InvokeCallback>, notifier: UpdateNotifier) -> Self {
        Self(Rc::new(LocalCallbackQueue { tx, notifier }))
    }
}

pub struct LocalCallbackQueue {
    tx: mpsc::Sender<InvokeCallback>,
    notifier: UpdateNotifier,
}

impl LocalCallbackQueue {
    /// # Panics
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any>) {
        self.tx.send(InvokeCallback { callback_id, arg }).ok();
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
    notifier: UpdateNotifier,
}

impl SharedCallbackQueue {
    /// # Panics
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any + Send>) {
        self.tx.send(InvokeCallback { callback_id, arg }).ok();
        self.notifier.notify();
    }
}

pub(crate) struct InvokeCallback {
    pub callback_id: CallbackId,
    pub arg: Box<dyn Any>,
}
