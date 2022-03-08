use std::{any::Any, collections::hash_set::Iter};

use fnv::FnvHashSet;

use crate::{
    state::ListenerId,
    widget::callback::{Callback, CallbackId},
};

#[derive(Default)]
pub struct Notifier {
    pub(crate) changed: FnvHashSet<ListenerId>,
    pub(crate) callbacks: Vec<(CallbackId, Box<dyn Any>)>,
}

impl Notifier {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.callbacks.is_empty()
    }

    pub fn notify(&mut self, listener_id: ListenerId) {
        self.changed.insert(listener_id);
    }

    pub fn notify_many(&mut self, listener_ids: Iter<'_, ListenerId>) {
        self.changed.extend(listener_ids);
    }

    pub fn emit<A>(&mut self, callback: Callback<A>, args: A)
    where
        A: 'static,
    {
        if let Some(callback_id) = callback.get_id() {
            self.callbacks.push((callback_id, Box::new(args)));
        }
    }
}
