use std::{any::Any, sync::Arc};

use parking_lot::Mutex;

use crate::unit::AsAny;

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
        A: AsAny,
    {
        self.queue.lock().extend(match callback {
            Callback::None => None,

            Callback::Func(func) => Some(CallbackInvoke::Func {
                func: Box::new(move || func.call(arg)),
            }),

            Callback::Widget(callback) => Some(CallbackInvoke::Widget {
                callback_id: callback.get_id(),
                arg: Box::new(arg),
            }),
        });
    }

    pub fn call_many<'a, A>(&self, callbacks: impl IntoIterator<Item = &'a Callback<A>>, arg: A)
    where
        A: AsAny + Clone,
    {
        self.queue
            .lock()
            .extend(callbacks.into_iter().filter_map(|callback| match callback {
                Callback::None => None,

                Callback::Func(func) => Some(CallbackInvoke::Func {
                    func: Box::new({
                        let func = func.clone();
                        let arg = arg.clone();

                        move || func.call(arg)
                    }),
                }),

                Callback::Widget(callback) => Some(CallbackInvoke::Widget {
                    callback_id: callback.get_id(),
                    arg: Box::new(arg.clone()),
                }),
            }));
    }

    /// # Panics
    ///
    /// This function must be called with the expected `arg` for the `callback_id`, or it will panic.
    pub fn call_unchecked(&self, callback_id: CallbackId, arg: Box<dyn Any>) {
        self.queue
            .lock()
            .push(CallbackInvoke::Widget { callback_id, arg });
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
                    .map(|callback_id| CallbackInvoke::Widget {
                        callback_id,
                        arg: Box::new(arg.clone()),
                    }),
            );
    }
}

pub(crate) enum CallbackInvoke {
    Widget {
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    },
    Func {
        func: Box<dyn FnOnce()>,
    },
}
