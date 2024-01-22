use std::{hash::BuildHasherDefault, sync::Arc};

use parking_lot::Mutex;
use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

use crate::engine::update_notifier::UpdateNotifier;

pub struct Dirty<T>
where
    T: slotmap::Key,
{
    inner: Arc<Mutex<SparseSecondaryMap<T, (), BuildHasherDefault<FxHasher>>>>,
    notifier: UpdateNotifier,
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
    pub(crate) fn new(notifier: UpdateNotifier) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SparseSecondaryMap::default())),
            notifier,
        }
    }

    /// Marks an entry as dirty, causing it to be processed at the next opportunity.
    pub fn insert(&self, key: T) {
        self.inner.lock().insert(key, ());
    }

    /// Notify the executor that there are dirty entries. This should only be called
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
        let mut inner = self.inner.lock();

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
