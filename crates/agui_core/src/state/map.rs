use std::{any::TypeId, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};
use parking_lot::Mutex;

use crate::engine::ChangedListeners;

use super::ListenerId;

use super::{State, StateValue};

struct StateEntry {
    value: Arc<dyn StateValue>,
    updated_value: Arc<Mutex<Option<Arc<dyn StateValue>>>>,

    listeners: FnvHashSet<ListenerId>,
}

pub struct StateMap {
    changed_listeners: ChangedListeners,

    entries: FnvHashMap<TypeId, StateEntry>,
}

impl StateMap {
    pub fn new(changed_listeners: ChangedListeners) -> Self {
        Self {
            changed_listeners,

            entries: FnvHashMap::default(),
        }
    }

    pub fn apply_updates(&mut self) {
        for (.., entry) in self.entries.iter_mut() {
            if let Some(value) = entry.updated_value.lock().take() {
                entry.value = value;

                self.changed_listeners.notify_many(entry.listeners.iter());
            }
        }
    }

    pub fn try_get<V>(&mut self, listener_id: Option<ListenerId>) -> Option<State<V>>
    where
        V: StateValue,
    {
        if let Some(entry) = self.entries.get_mut(&TypeId::of::<V>()) {
            {
                let mut updated_value = entry.updated_value.lock();

                if let Some(updated_value) = updated_value.take() {
                    entry.value = updated_value;
                }
            }

            if let Some(listener_id) = listener_id {
                entry.listeners.insert(listener_id);
            }

            Some(State {
                value: Arc::clone(&entry.value)
                    .downcast_arc()
                    .expect("state failed to downcast ref"),
                updated_value: Arc::clone(&entry.updated_value),
            })
        } else {
            None
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn get_or<V, F>(&mut self, listener_id: Option<ListenerId>, func: F) -> State<V>
    where
        V: StateValue,
        F: FnOnce() -> V,
    {
        if let Some(state) = self.try_get::<V>(listener_id) {
            state
        } else {
            let type_id = TypeId::of::<V>();

            self.entries.insert(
                type_id,
                StateEntry {
                    value: Arc::new(func()),
                    updated_value: Arc::default(),

                    listeners: FnvHashSet::default(),
                },
            );

            self.try_get::<V>(listener_id)
                .expect("did not properly insert state")
        }
    }

    pub fn remove_listeners(&mut self, listener_id: &ListenerId) {
        for entry in self.entries.values_mut() {
            entry.listeners.remove(listener_id);
        }
    }
}
