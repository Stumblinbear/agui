use std::{any::Any, sync::Arc};

use parking_lot::Mutex;

use crate::unit::AsAny;

use super::CallbackId;

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

    /// # Panics
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any>) {
        self.queue.lock().push(CallbackInvoke { callback_id, arg });
    }

    /// # Panics
    ///
    /// This function must be called with the expected `arg` for all of the `callback_ids`, or it will panic.
    pub fn call_many_unchecked<'a, A>(
        &self,
        callback_ids: impl IntoIterator<Item = &'a CallbackId>,
        arg: A,
    ) where
        A: AsAny + Clone,
    {
        self.queue
            .lock()
            .extend(
                callback_ids
                    .into_iter()
                    .copied()
                    .map(|callback_id| CallbackInvoke {
                        callback_id,
                        arg: Box::new(arg.clone()),
                    }),
            );
    }
}

pub(crate) struct CallbackInvoke {
    pub callback_id: CallbackId,
    pub arg: Box<dyn Any>,
}
