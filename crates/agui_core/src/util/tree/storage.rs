use slotmap::{HopSlotMap, SparseSecondaryMap};

use crate::util::tree::storage::sealed::TreeContainer;

pub(super) mod sealed {
    pub trait TreeContainer<K, V>
    where
        K: Copy + PartialEq,
    {
        type Iter<'a>: Iterator<Item = (K, &'a V)>
        where
            K: 'a,
            V: 'a,
            Self: 'a;

        fn iter(&self) -> Self::Iter<'_>;

        fn contains_key(&self, key: K) -> bool;

        fn get(&self, key: K) -> Option<&V>;

        fn get_mut(&mut self, key: K) -> Option<&mut V>;

        fn remove(&mut self, key: K) -> Option<V>;

        fn clear(&mut self);

        /// Returns if the tree is empty.
        fn is_empty(&self) -> bool;

        /// Returns the number of nodes in the tree.
        fn len(&self) -> usize;
    }

    pub trait TreeContainerAdd<K, V>
    where
        K: Copy + PartialEq,
    {
        fn add(&mut self, value: V) -> K;
    }

    pub trait TreeContainerInsert<K, V>
    where
        K: Copy + PartialEq,
    {
        fn insert(&mut self, key: K, value: V);
    }
}

pub trait TreeStorage {
    type Container<K, V>: TreeContainer<K, V>
    where
        K: slotmap::Key;
}

pub struct HopSlotMapStorage;

impl TreeStorage for HopSlotMapStorage {
    type Container<K, V> = HopSlotMap<K, V>
    where
        K: slotmap::Key;
}

impl<K, V> sealed::TreeContainer<K, V> for HopSlotMap<K, V>
where
    K: slotmap::Key,
{
    type Iter<'a> = slotmap::hop::Iter<'a, K, V> where K: 'a, V: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        HopSlotMap::iter(self)
    }

    fn contains_key(&self, key: K) -> bool {
        HopSlotMap::contains_key(self, key)
    }

    fn get(&self, key: K) -> Option<&V> {
        HopSlotMap::get(self, key)
    }

    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        HopSlotMap::get_mut(self, key)
    }

    fn remove(&mut self, key: K) -> Option<V> {
        HopSlotMap::remove(self, key)
    }

    fn clear(&mut self) {
        HopSlotMap::clear(self)
    }

    fn is_empty(&self) -> bool {
        HopSlotMap::is_empty(self)
    }

    fn len(&self) -> usize {
        HopSlotMap::len(self)
    }
}

impl<K, V> sealed::TreeContainerAdd<K, V> for HopSlotMap<K, V>
where
    K: slotmap::Key,
{
    fn add(&mut self, value: V) -> K {
        HopSlotMap::insert(self, value)
    }
}

pub struct SparseSecondaryMapStorage;

impl TreeStorage for SparseSecondaryMapStorage {
    type Container<K, V> = SparseSecondaryMap<K, V>
    where
        K: slotmap::Key;
}

impl<K, V> sealed::TreeContainer<K, V> for SparseSecondaryMap<K, V>
where
    K: slotmap::Key,
{
    type Iter<'a> = slotmap::sparse_secondary::Iter<'a, K, V> where K: 'a, V: 'a;

    fn iter(&self) -> Self::Iter<'_> {
        SparseSecondaryMap::iter(self)
    }

    fn contains_key(&self, key: K) -> bool {
        SparseSecondaryMap::contains_key(self, key)
    }

    fn get(&self, key: K) -> Option<&V> {
        SparseSecondaryMap::get(self, key)
    }

    fn get_mut(&mut self, key: K) -> Option<&mut V> {
        SparseSecondaryMap::get_mut(self, key)
    }

    fn remove(&mut self, key: K) -> Option<V> {
        SparseSecondaryMap::remove(self, key)
    }

    fn clear(&mut self) {
        SparseSecondaryMap::clear(self)
    }

    fn is_empty(&self) -> bool {
        SparseSecondaryMap::is_empty(self)
    }

    fn len(&self) -> usize {
        SparseSecondaryMap::len(self)
    }
}

impl<K, V> sealed::TreeContainerInsert<K, V> for SparseSecondaryMap<K, V>
where
    K: slotmap::Key,
{
    fn insert(&mut self, key: K, value: V) {
        SparseSecondaryMap::insert(self, key, value);
    }
}
