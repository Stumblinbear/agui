use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

pub struct Dirty<T>
where
    T: slotmap::Key,
{
    inner: SparseSecondaryMap<T, (), BuildHasherDefault<FxHasher>>,
}

impl<T> Default for Dirty<T>
where
    T: slotmap::Key,
{
    fn default() -> Self {
        Self {
            inner: SparseSecondaryMap::default(),
        }
    }
}

impl<T> Dirty<T>
where
    T: slotmap::Key,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any any entries have been added.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Check if a given key exists in the list.
    pub fn is_dirty(&self, key: T) -> bool {
        self.inner.contains_key(key)
    }

    /// Marks an entry as dirty, causing it to be processed at the next opportunity.
    pub fn insert(&mut self, key: T) {
        self.inner.insert(key, ());
    }

    pub(super) fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
        self.inner.drain().map(|(key, _)| key)
    }
}
