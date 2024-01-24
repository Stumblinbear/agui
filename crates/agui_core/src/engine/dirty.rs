use std::{
    hash::BuildHasherDefault,
    sync::{Arc, Mutex},
};

use agui_sync::notify;
use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

pub struct Dirty<T>
where
    T: slotmap::Key,
{
    inner: Arc<Mutex<SparseSecondaryMap<T, (), BuildHasherDefault<FxHasher>>>>,
    notifier: notify::Flag,
}

impl<T> Clone for Dirty<T>
where
    T: slotmap::Key,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            notifier: self.notifier.clone(),
        }
    }
}

impl<T> Dirty<T>
where
    T: slotmap::Key,
{
    pub(crate) fn new(notifier: notify::Flag) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SparseSecondaryMap::default())),
            notifier,
        }
    }

    /// Marks an entry as dirty, causing it to be processed at the next opportunity.
    pub fn insert(&self, key: T) {
        self.inner
            .lock()
            .expect("dirty mutex poisoned")
            .insert(key, ());
    }

    /// Notify listeners that there are dirty entries. This should only be called
    /// when we're not already in the middle of an update.
    pub fn notify(&self) {
        self.notifier.notify();
    }

    /// Drains the dirty entries, processing them with the given function.
    ///
    /// Returns `true` if the queue was not empty.
    pub(super) fn process<F>(&mut self, mut func: F) -> bool
    where
        F: FnMut(T),
    {
        let mut inner = self.inner.lock().expect("dirty mutex poisoned");

        if inner.is_empty() {
            false
        } else {
            for (key, _) in inner.drain() {
                func(key);
            }

            true
        }
    }
}
