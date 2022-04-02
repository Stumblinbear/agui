use std::{cell::RefCell, collections::hash_set::Iter, sync::Arc};

use fnv::FnvHashSet;
use parking_lot::Mutex;

use crate::{
    state::{ListenerId, Data},
    widget::callback::CallbackId,
};

pub type NotifyChanged = RefCell<FnvHashSet<ListenerId>>;
pub type NotifyCallback = Arc<Mutex<Vec<(CallbackId, Box<dyn Data>)>>>;

#[derive(Default)]
pub struct Notifier {
    pub(crate) changed: NotifyChanged,
    pub(crate) callbacks: NotifyCallback,
}

impl Notifier {
    pub fn is_empty(&self) -> bool {
        self.changed.borrow().is_empty() && self.callbacks.lock().is_empty()
    }

    pub fn notify(&self, listener_id: ListenerId) {
        self.changed.borrow_mut().insert(listener_id);
    }

    pub fn notify_many(&self, listener_ids: Iter<'_, ListenerId>) {
        self.changed.borrow_mut().extend(listener_ids);
    }
}
