use std::{hash::Hash, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};
use parking_lot::{Mutex, RwLock};

use crate::context::ListenerId;

pub struct ReadableMap<K, V>
where
    K: Hash + Eq,
{
    values: FnvHashMap<K, V>,

    listeners: RwLock<FnvHashMap<K, FnvHashSet<ListenerId>>>,

    changed: Arc<Mutex<FnvHashSet<ListenerId>>>,
}

impl<K, V> ReadableMap<K, V>
where
    K: Hash + Eq,
{
    pub fn new(changed: Arc<Mutex<FnvHashSet<ListenerId>>>) -> Self {
        Self {
            values: FnvHashMap::default(),

            listeners: RwLock::default(),

            changed,
        }
    }

    pub fn set(&mut self, key: K, value: V) {
        if let Some(listeners) = self.listeners.read().get(&key) {
            self.changed.lock().extend(listeners);
        }

        self.values.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.values.get(key)
    }

    pub fn remove(&mut self, key: &K) {
        self.values.remove(key);

        self.listeners.write().remove(key);
    }

    pub fn add_listener(&self, key: K, listener_id: ListenerId) {
        let mut listeners = self.listeners.write();

        let notify = listeners.entry(key).or_insert_with(FnvHashSet::default);

        notify.insert(listener_id);
    }

    pub fn remove_listeners(&self, listener_id: &ListenerId) {
        for notify in self.listeners.write().values_mut() {
            notify.remove(listener_id);
        }
    }
}
