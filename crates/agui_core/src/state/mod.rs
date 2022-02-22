use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use downcast_rs::{impl_downcast, DowncastSync};
use parking_lot::Mutex;

mod listener;
pub(crate) mod map;

pub use listener::ListenerId;

pub trait StateValue: std::fmt::Debug + DowncastSync + Send + Sync + 'static {}

impl<T> StateValue for T where T: std::fmt::Debug + Send + Sync + 'static {}

impl_downcast!(sync StateValue);

/// Holds the state of a value, with notify-on-write.
pub struct State<V>
where
    V: StateValue,
{
    value: Arc<V>,
    updated_value: Arc<Mutex<Option<Arc<dyn StateValue>>>>,
}

impl<V> State<V>
where
    V: StateValue,
{
    /// Write to the state.
    ///
    /// This will trigger an update of any components listening to the state. Use only if something legitimately changes.
    pub fn set(&self, value: V) {
        *self.updated_value.lock() = Some(Arc::new(value));
    }
}

impl<V> Clone for State<V>
where
    V: StateValue,
{
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            updated_value: Arc::clone(&self.updated_value),
        }
    }
}

impl<V> std::fmt::Display for State<V>
where
    V: StateValue + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<V> std::fmt::Debug for State<V>
where
    V: StateValue + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl<V> Deref for State<V>
where
    V: StateValue,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V> State<V>
where
    V: StateValue + Clone,
{
    pub fn write(&self) -> Write<V> {
        Write {
            value: self.value.as_ref().clone(),
            updated_value: Arc::clone(&self.updated_value),
        }
    }
}

pub struct Write<V>
where
    V: StateValue + Clone,
{
    value: V,
    updated_value: Arc<Mutex<Option<Arc<dyn StateValue>>>>,
}

impl<V> Deref for Write<V>
where
    V: StateValue + Clone,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V> DerefMut for Write<V>
where
    V: StateValue + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<V> Drop for Write<V>
where
    V: StateValue + Clone,
{
    fn drop(&mut self) {
        *self.updated_value.lock() = Some(Arc::new(self.value.clone()));
    }
}
