use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    hash::Hash,
    marker::PhantomData,
    sync::Arc,
};

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use super::{ListenerID, Value};

pub struct NotifiableValue {
    value: Option<Arc<RwLock<Box<dyn Value>>>>,

    notify: Arc<Mutex<HashSet<ListenerID>>>,
}

pub struct NotifiableMap {
    values: RwLock<HashMap<TypeId, NotifiableValue>>,

    changed: Arc<Mutex<HashSet<ListenerID>>>,
}

impl NotifiableMap {
    // #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    pub fn new(changed: Arc<Mutex<HashSet<ListenerID>>>) -> Self {
        Self {
            values: RwLock::default(),

            changed,
        }
    }

    fn ensure_value<V>(&self)
    where
        V: Value,
    {
        let mut values = self.values.write();

        values.entry(TypeId::of::<V>()).or_insert_with(|| {
            let notify = Arc::new(Mutex::new(HashSet::new()));

            NotifiableValue {
                value: None,

                notify: Arc::clone(&notify),
            }
        });
    }

    pub fn contains<V>(&self) -> bool
    where
        V: Value,
    {
        self.values
            .read()
            .get(&TypeId::of::<V>())
            .map_or(false, |state| state.value.is_some())
    }

    pub fn insert<V>(&self, value: V)
    where
        V: Value,
    {
        self.ensure_value::<V>();

        let mut values = self.values.write();

        let notifiable = values.get_mut(&TypeId::of::<V>()).unwrap();

        notifiable.value = Some(Arc::new(RwLock::new(Box::new(value))));
    }

    pub fn get<V>(&self) -> Option<Notify<V>>
    where
        V: Value,
    {
        if let Some(notifiable) = self.values.read().get(&TypeId::of::<V>()) {
            if let Some(value) = &notifiable.value {
                return Some(Notify {
                    phantom: PhantomData,

                    notify: Arc::clone(&notifiable.notify),
                    changed: Arc::clone(&self.changed),

                    value: Arc::clone(value),
                });
            }
        }

        None
    }

    pub fn add_listener<V>(&self, listener_id: ListenerID)
    where
        V: Value,
    {
        self.ensure_value::<V>();

        let mut values = self.values.write();

        let notifiable = values.get_mut(&TypeId::of::<V>()).unwrap();

        notifiable.notify.lock().insert(listener_id);
    }

    pub fn remove_listener(&self, listener_id: &ListenerID) {
        for notifiable in self.values.write().values() {
            notifiable.notify.lock().remove(listener_id);
        }
    }
}

pub struct ScopedNotifiableMap<K>
where
    K: Eq + Hash,
{
    scopes: Mutex<HashMap<K, NotifiableMap>>,

    changed: Arc<Mutex<HashSet<ListenerID>>>,
}

impl<K> ScopedNotifiableMap<K>
where
    K: Eq + Hash,
{
    #[must_use]
    pub fn new(changed: Arc<Mutex<HashSet<ListenerID>>>) -> Self {
        Self {
            scopes: Mutex::default(),
            changed,
        }
    }

    pub fn init<V, F>(&self, key: K, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let mut scopes = self.scopes.lock();

        let scope = scopes
            .entry(key)
            .or_insert_with(|| NotifiableMap::new(Arc::clone(&self.changed)));

        if !scope.contains::<V>() {
            scope.insert(func());
        }

        scope.get().expect("failed to get scope")
    }

    pub fn set<V>(&self, key: K, value: V) -> Notify<V>
    where
        V: Value,
    {
        let mut scopes = self.scopes.lock();

        let scope = scopes
            .entry(key)
            .or_insert_with(|| NotifiableMap::new(Arc::clone(&self.changed)));

        scope.insert(value);

        scope.get().expect("failed to get scope")
    }

    pub fn get<V, F>(&self, key: K, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let mut scopes = self.scopes.lock();

        let scope = scopes
            .entry(key)
            .or_insert_with(|| NotifiableMap::new(Arc::clone(&self.changed)));

        if !scope.contains::<V>() {
            scope.insert(func());
        }

        // scope.add_listener::<V>(*listener_id);

        scope.get().expect("failed to get state")
    }

    pub fn remove(&self, key: &K) {
        self.scopes.lock().remove(key);
    }

    pub fn add_listener<V>(&self, key: K, listener_id: ListenerID)
    where
        V: Value,
    {
        let mut scopes = self.scopes.lock();

        let scope = scopes
            .entry(key)
            .or_insert_with(|| NotifiableMap::new(Arc::clone(&self.changed)));

        scope.add_listener::<V>(listener_id);
    }

    pub fn remove_listeners(&self, listener_id: &ListenerID) {
        self.scopes
            .lock()
            .iter()
            .for_each(|(_, states)| states.remove_listener(listener_id));
    }
}

/// Holds the state of a value, with notify-on-write.
pub struct Notify<V>
where
    V: Value,
{
    pub(crate) phantom: PhantomData<V>,

    pub(crate) notify: Arc<Mutex<HashSet<ListenerID>>>,
    pub(crate) changed: Arc<Mutex<HashSet<ListenerID>>>,

    pub(crate) value: Arc<RwLock<Box<dyn Value>>>,
}

impl<V> Clone for Notify<V>
where
    V: Value,
{
    fn clone(&self) -> Self {
        Self {
            phantom: self.phantom,

            notify: Arc::clone(&self.notify),
            changed: Arc::clone(&self.changed),

            value: Arc::clone(&self.value),
        }
    }
}

#[allow(clippy::missing_panics_doc)]
impl<V> Notify<V>
where
    V: Value,
{
    /// Read the state.
    pub fn read(&self) -> MappedRwLockReadGuard<V> {
        RwLockReadGuard::map(self.value.read(), |value| {
            value
                .downcast_ref::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }

    /// Write to the state.
    ///
    /// This will trigger an update of any components listening to the state. Use only if something legitimately changes.
    pub fn write(&self) -> MappedRwLockWriteGuard<V> {
        self.changed.lock().extend(self.notify.lock().iter());

        RwLockWriteGuard::map(self.value.write(), |value| {
            value
                .downcast_mut::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }
}
