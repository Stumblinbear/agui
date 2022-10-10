use std::sync::Arc;

use parking_lot::Mutex;

use crate::unit::Data;

use super::{Callback, CallbackId};

#[derive(Default, Clone)]
pub struct CallbackQueue {
    queue: Arc<Mutex<Vec<CallbackInvoke>>>,
}

impl CallbackQueue {
    pub(crate) fn take(&mut self) -> Vec<CallbackInvoke> {
        self.queue.lock().drain(..).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    pub fn call<A>(&self, callback: Callback<A>, arg: A)
    where
        A: Data,
    {
        if let Some(callback_id) = callback.get_id() {
            self.queue.lock().push(CallbackInvoke {
                callback_ids: vec![callback_id],
                arg: Box::new(arg),
            });
        }
    }

    pub fn call_many<A>(&self, callbacks: &[Callback<A>], arg: A)
    where
        A: Data,
    {
        self.queue.lock().push(CallbackInvoke {
            callback_ids: callbacks.iter().filter_map(|id| id.get_id()).collect(),
            arg: Box::new(arg),
        });
    }

    /// # Safety
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub unsafe fn call_unsafe(&self, callback_id: CallbackId, arg: Box<dyn Data>) {
        self.queue.lock().push(CallbackInvoke {
            callback_ids: vec![callback_id],
            arg,
        });
    }

    /// # Safety
    ///
    /// This function must be called with the expected `arg` for all of the `callback_ids`, or it will panic.
    pub unsafe fn call_many_unsafe(&self, callback_ids: &[CallbackId], arg: Box<dyn Data>) {
        self.queue.lock().push(CallbackInvoke {
            callback_ids: Vec::from(callback_ids),
            arg,
        });
    }
}

pub(crate) struct CallbackInvoke {
    pub callback_ids: Vec<CallbackId>,
    pub arg: Box<dyn Data>,
}
