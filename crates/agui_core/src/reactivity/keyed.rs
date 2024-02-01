use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

use crate::unit::Key;

#[derive(Default)]
pub struct KeyMap<K>
where
    K: slotmap::Key,
{
    map: SparseSecondaryMap<K, Key, BuildHasherDefault<FxHasher>>,
    // from_global_keys: FxHashMap<u64, K>,
}

impl<K> KeyMap<K>
where
    K: slotmap::Key,
{
    pub fn get_key(&self, node_id: K) -> Option<Key> {
        self.map.get(node_id).copied()
    }

    pub fn get_element(&self, key: Key) -> Option<K> {
        // if let Key::Global(key_data) = key {
        //     self.from_global_keys.get(&key_data).copied()
        // } else {
        //     None
        // }

        None
    }

    pub(super) fn insert(&mut self, node_id: K, key: Key) {
        self.map.insert(node_id, key);

        // if let Key::Global(key_data) = key {
        //     self.from_global_keys.insert(key_data, node_id);
        // }
    }

    pub(super) fn remove(&mut self, node_id: K) {
        self.map.remove(node_id);

        // if let Some(Key::Global(key_data)) = self.map.remove(node_id) {
        //     self.from_global_keys.remove(&key_data);
        // }
    }
}
