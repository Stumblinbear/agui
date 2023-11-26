use std::hash::Hash;

use rustc_hash::FxHashSet;

pub struct Dirty<T> {
    inner: FxHashSet<T>,
}

impl<T> Dirty<T>
where
    T: PartialEq + Eq + Hash,
{
    pub(super) fn new() -> Self {
        Self {
            inner: FxHashSet::default(),
        }
    }

    /// Check if any any entries have been added.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Check if a given key exists in the list.
    pub fn is_dirty(&self, key: &T) -> bool {
        self.inner.contains(key)
    }

    /// Marks an entry as dirty, causing it to be processed at the next opportunity.
    pub fn insert(&mut self, key: T) {
        self.inner.insert(key);
    }

    pub(super) fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.inner.drain()
    }
}
