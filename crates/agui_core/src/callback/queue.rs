use std::{any::Any, sync::Arc};

use parking_lot::Mutex;

use crate::{engine::update_notifier::UpdateNotifier, unit::AsAny};

use super::CallbackId;

#[derive(Clone)]
pub struct CallbackQueue(Arc<InnerCallbackQueue>);

struct InnerCallbackQueue {
    queue: Mutex<Vec<CallbackInvoke>>,
    notifier: UpdateNotifier,
}

impl CallbackQueue {
    pub(crate) fn new(notifier: UpdateNotifier) -> Self {
        Self(Arc::new(InnerCallbackQueue {
            queue: Mutex::default(),
            notifier,
        }))
    }

    pub(crate) fn take(&mut self) -> Vec<CallbackInvoke> {
        self.0.queue.lock().drain(..).collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.queue.lock().is_empty()
    }

    /// # Panics
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any>) {
        self.0
            .queue
            .lock()
            .push(CallbackInvoke { callback_id, arg });

        self.0.notifier.notify();
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
        self.0
            .queue
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

        self.0.notifier.notify();
    }
}

pub(crate) struct CallbackInvoke {
    pub callback_id: CallbackId,
    pub arg: Box<dyn Any>,
}
