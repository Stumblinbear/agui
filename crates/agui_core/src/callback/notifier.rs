use std::any::TypeId;

use crate::element::ElementId;

use super::{CallbackId, CallbackQueue};

pub struct Notifier {
    id: CallbackId,

    callback_queue: CallbackQueue,
}

impl Notifier {
    pub(crate) fn new(element_id: ElementId, callback_queue: CallbackQueue) -> Self {
        Self {
            id: CallbackId {
                element_id,
                type_id: TypeId::of::<()>(),
            },

            callback_queue,
        }
    }

    pub fn notify(&self) {
        self.callback_queue.call_unchecked(self.id, Box::new(()));
    }
}
