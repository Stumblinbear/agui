use std::{any::TypeId, hash::Hash, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};
use parking_lot::{Mutex, RwLock};

use super::ListenerId;

use super::{NotifiableValue, Notify};

pub struct StateMap {
    values: RwLock<FnvHashMap<TypeId, Notify<Box<dyn NotifiableValue>>>>,

    changed: Arc<Mutex<FnvHashSet<ListenerId>>>,
}

impl StateMap {
    pub fn new(changed: Arc<Mutex<FnvHashSet<ListenerId>>>) -> Self {
        Self {
            values: RwLock::default(),

            changed,
        }
    }

    pub fn try_get<V>(&self) -> Option<Notify<V>>
    where
        V: NotifiableValue,
    {
        let mut values = self.values.write();

        let notify = values
            .entry(TypeId::of::<V>())
            .or_insert_with(|| Notify::new(Arc::clone(&self.changed)));

        if notify.value.read().is_some() {
            Some(notify.cast())
        } else {
            None
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn get_or<V, F>(&self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        let notify = self
            .values
            .write()
            .entry(TypeId::of::<V>())
            .or_insert_with(|| Notify::new(Arc::clone(&self.changed)))
            .cast();

        if notify.value.read().is_none() {
            *notify.value.write() = Some(Box::new(func()));
        }

        notify
    }

    pub fn add_listener<V>(&self, listener_id: ListenerId)
    where
        V: NotifiableValue,
    {
        let mut values = self.values.write();

        let notify = values
            .entry(TypeId::of::<V>())
            .or_insert_with(|| Notify::new(Arc::clone(&self.changed)));

        notify.listeners.lock().insert(listener_id);
    }

    pub fn remove_listeners(&self, listener_id: &ListenerId) {
        for notify in self.values.write().values() {
            notify.listeners.lock().remove(listener_id);
        }
    }
}

pub struct ScopedStateMap<K>
where
    K: Eq + Hash,
{
    scopes: Mutex<FnvHashMap<K, StateMap>>,

    changed: Arc<Mutex<FnvHashSet<ListenerId>>>,
}

impl<K> ScopedStateMap<K>
where
    K: Eq + Hash,
{
    pub fn new(changed: Arc<Mutex<FnvHashSet<ListenerId>>>) -> Self {
        Self {
            scopes: Mutex::default(),

            changed,
        }
    }

    pub fn get<V, F>(&self, key: K, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        let mut scopes = self.scopes.lock();

        let scope = scopes
            .entry(key)
            .or_insert_with(|| StateMap::new(Arc::clone(&self.changed)));

        scope.get_or(func)
    }

    pub fn remove(&self, key: &K) {
        self.scopes.lock().remove(key);
    }

    pub fn add_listener<V>(&self, key: K, listener_id: ListenerId)
    where
        V: NotifiableValue,
    {
        let mut scopes = self.scopes.lock();

        let scope = scopes
            .entry(key)
            .or_insert_with(|| StateMap::new(Arc::clone(&self.changed)));

        scope.add_listener::<V>(listener_id);
    }

    pub fn remove_listeners(&self, listener_id: &ListenerId) {
        self.scopes
            .lock()
            .iter()
            .for_each(|(_, states)| states.remove_listeners(listener_id));
    }
}
