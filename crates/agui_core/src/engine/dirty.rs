use std::{
    hash::BuildHasherDefault,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use agui_sync::notify;
use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

pub struct Dirty<T>(Arc<Inner<T>>)
where
    T: slotmap::Key;

struct Inner<T>
where
    T: slotmap::Key,
{
    entries: Mutex<SparseSecondaryMap<T, (), BuildHasherDefault<FxHasher>>>,
    has_entries: AtomicBool,
    notifier: notify::Flag,
}

impl<T> Clone for Dirty<T>
where
    T: slotmap::Key,
{
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Dirty<T>
where
    T: slotmap::Key,
{
    pub(crate) fn new(notifier: notify::Flag) -> Self {
        Self(Arc::new(Inner {
            entries: Mutex::new(SparseSecondaryMap::default()),
            has_entries: AtomicBool::new(false),
            notifier,
        }))
    }

    pub fn is_empty(&self) -> bool {
        !self.0.has_entries.load(Ordering::Relaxed)
    }

    /// Marks an entry as dirty, causing it to be processed at the next opportunity.
    pub fn insert(&self, key: T) {
        self.0
            .entries
            .lock()
            .expect("dirty mutex poisoned")
            .insert(key, ());

        self.0.has_entries.store(true, Ordering::Relaxed);
    }

    /// Notify listeners that there are dirty entries. This should ideally only be
    /// called if we're not already in the middle of an update.
    pub fn notify(&self) {
        self.0.notifier.notify();
    }

    /// Drains the dirty entries, processing them with the given function.
    ///
    /// Returns `true` if the queue was not empty.
    pub(super) fn process<F>(&mut self, mut func: F) -> bool
    where
        F: FnMut(T),
    {
        let mut inner = self.0.entries.lock().expect("dirty mutex poisoned");

        self.0.has_entries.store(false, Ordering::Relaxed);

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
