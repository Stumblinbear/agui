use std::collections::hash_map::Entry;
use std::{any::TypeId, cell::RefCell, rc::Rc};

use fnv::{FnvHashMap, FnvHashSet};

use crate::engine::notify::Notifier;

use super::ListenerId;

use super::{State, Data};

struct StateEntry {
    value: Rc<dyn Data>,
    updated_value: Rc<RefCell<Option<Rc<dyn Data>>>>,
}

pub struct StateMap {
    notifier: Rc<Notifier>,

    entries: FnvHashMap<TypeId, StateEntry>,
    listeners: FnvHashMap<TypeId, Rc<RefCell<FnvHashSet<ListenerId>>>>,
}

impl StateMap {
    pub fn new(notifier: Rc<Notifier>) -> Self {
        Self {
            notifier,

            entries: FnvHashMap::default(),
            listeners: FnvHashMap::default(),
        }
    }

    pub fn try_get<V>(&mut self, listener_id: Option<ListenerId>) -> Option<State<V>>
    where
        V: Data + Clone,
    {
        let type_id = TypeId::of::<V>();

        let listeners = self.listeners.entry(type_id).or_default();

        if let Some(listener_id) = listener_id {
            listeners.borrow_mut().insert(listener_id);
        }

        self.entries.get_mut(&type_id).map(|entry| {
            if let Some(updated_value) = entry.updated_value.borrow_mut().take() {
                entry.value = updated_value;
            }

            State::new(
                Rc::clone(&self.notifier),
                Rc::clone(self.listeners.get(&type_id).unwrap()),
                Rc::clone(&entry.value)
                    .downcast_rc()
                    .expect("failed to downcast ref"),
                Rc::clone(&entry.updated_value),
            )
        })
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn get_or<V, F>(&mut self, listener_id: Option<ListenerId>, func: F) -> State<V>
    where
        V: Data + Clone,
        F: FnOnce() -> V,
    {
        self.entries
            .entry(TypeId::of::<V>())
            .or_insert_with(|| StateEntry {
                value: Rc::new(func()),
                updated_value: Rc::default(),
            });

        self.try_get::<V>(listener_id)
            .expect("did not properly insert state")
    }

    pub fn set<V>(&mut self, value: V)
    where
        V: Data + Clone,
    {
        let type_id = TypeId::of::<V>();

        if let Entry::Vacant(e) = self.entries.entry(type_id) {
            e.insert(StateEntry {
                value: Rc::new(value),
                updated_value: Rc::default(),
            });
        } else {
            let entry = self.entries.get_mut(&type_id).unwrap();

            entry.updated_value.borrow_mut().replace(Rc::new(value));
        }

        if let Some(listeners) = self.listeners.get(&type_id) {
            self.notifier.notify_many(listeners.borrow().iter());
        }
    }

    pub fn remove_listeners(&mut self, listener_id: &ListenerId) {
        for entry in self.listeners.values_mut() {
            entry.borrow_mut().remove(listener_id);
        }
    }
}
