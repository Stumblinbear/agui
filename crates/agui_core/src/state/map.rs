use std::{any::TypeId, cell::RefCell, rc::Rc};

use fnv::{FnvHashMap, FnvHashSet};

use crate::engine::notify::Notifier;

use super::ListenerId;

use super::{State, StateValue};

struct StateEntry {
    value: Rc<dyn StateValue>,
    updated_value: Rc<RefCell<Option<Rc<dyn StateValue>>>>,
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
        V: StateValue + Clone,
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
        V: StateValue + Clone,
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

    pub fn set<V>(&mut self, value: V) -> State<V>
    where
        V: StateValue + Clone,
    {
        self.get_or(None, || value)
    }

    pub fn remove_listeners(&mut self, listener_id: &ListenerId) {
        for entry in self.listeners.values_mut() {
            entry.borrow_mut().remove(listener_id);
        }
    }
}
