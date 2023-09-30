use std::{
    any::Any,
    sync::{mpsc, Arc},
};

use parking_lot::Mutex;

use crate::unit::AsAny;

use super::CallbackId;

#[derive(Clone)]
pub struct CallbackQueue(Arc<InnerCallbackQueue>);

struct InnerCallbackQueue {
    queue: Arc<Mutex<Vec<CallbackInvoke>>>,
    update_notifier_tx: mpsc::Sender<()>,
}

impl CallbackQueue {
    #[allow(clippy::arc_with_non_send_sync)]
    pub(crate) fn new(update_notifier_tx: mpsc::Sender<()>) -> Self {
        Self(Arc::new(InnerCallbackQueue {
            queue: Arc::default(),

            update_notifier_tx,
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

        let _ = self.0.update_notifier_tx.send(());
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

        let _ = self.0.update_notifier_tx.send(());
    }
}

pub(crate) struct CallbackInvoke {
    pub callback_id: CallbackId,
    pub arg: Box<dyn Any>,
}
