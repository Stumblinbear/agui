use std::{marker::PhantomData, sync::Arc};

use downcast_rs::{impl_downcast, Downcast};
use fnv::FnvHashSet;
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

mod listener;
pub mod state;

pub use listener::ListenerId;

pub trait NotifiableValue: std::fmt::Debug + Downcast + Send + Sync + 'static {}

impl<T> NotifiableValue for T where T: std::fmt::Debug + Send + Sync + 'static {}

impl_downcast!(NotifiableValue);

/// Holds the state of a value, with notify-on-write.
pub struct Notify<V>
where
    V: NotifiableValue,
{
    phantom: PhantomData<V>,

    value: Arc<RwLock<Option<Box<dyn NotifiableValue>>>>,

    listeners: Arc<Mutex<FnvHashSet<ListenerId>>>,

    changed: Arc<Mutex<FnvHashSet<ListenerId>>>,
}

#[allow(clippy::missing_panics_doc)]
impl<V> Notify<V>
where
    V: NotifiableValue,
{
    pub(crate) fn new(changed: Arc<Mutex<FnvHashSet<ListenerId>>>) -> Self {
        Self {
            phantom: PhantomData,

            value: Arc::default(),

            listeners: Arc::default(),

            changed,
        }
    }

    pub fn add_listener(&self, listener_id: ListenerId) {
        self.listeners.lock().insert(listener_id);
    }

    pub fn remove_listener(&self, listener_id: ListenerId) {
        self.listeners.lock().remove(&listener_id);
    }

    pub fn has_value(&self) -> bool {
        self.value.read().is_some()
    }

    pub fn set_value(&mut self, value: V) {
        *self.value.write() = Some(Box::new(value));
    }

    pub(crate) fn cast<N>(&self) -> Notify<N>
    where
        N: NotifiableValue,
    {
        Notify {
            phantom: PhantomData,

            value: Arc::clone(&self.value),

            listeners: Arc::clone(&self.listeners),

            changed: Arc::clone(&self.changed),
        }
    }

    /// Read the state.
    pub fn read(&self) -> MappedRwLockReadGuard<V> {
        RwLockReadGuard::map(self.value.read(), |value| {
            value
                .as_ref()
                .expect("unset notifiable value cannot be read")
                .downcast_ref::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }

    /// Write to the state.
    ///
    /// This will trigger an update of any components listening to the state. Use only if something legitimately changes.
    pub fn write(&self) -> MappedRwLockWriteGuard<V> {
        self.changed.lock().extend(self.listeners.lock().iter());

        RwLockWriteGuard::map(self.value.write(), |value| {
            value
                .as_mut()
                .expect("unset notifiable value cannot be written")
                .downcast_mut::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }
}

impl<V> Clone for Notify<V>
where
    V: NotifiableValue,
{
    fn clone(&self) -> Self {
        Self {
            phantom: self.phantom,

            value: Arc::clone(&self.value),

            listeners: Arc::clone(&self.listeners),

            changed: Arc::clone(&self.changed),
        }
    }
}
