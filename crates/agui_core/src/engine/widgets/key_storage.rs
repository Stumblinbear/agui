use std::hash::BuildHasherDefault;

use rustc_hash::{FxHashMap, FxHasher};
use slotmap::SparseSecondaryMap;

use crate::{element::ElementId, unit::Key};

#[derive(Default)]
pub struct WidgetKeyStorage {
    element_keys: SparseSecondaryMap<ElementId, Key, BuildHasherDefault<FxHasher>>,

    from_global_keys: FxHashMap<u64, ElementId>,
}

impl WidgetKeyStorage {
    pub fn get_key(&self, element_id: ElementId) -> Option<Key> {
        self.element_keys.get(element_id).copied()
    }

    pub fn get_element(&self, key: Key) -> Option<ElementId> {
        if let Key::Global(key_data) = key {
            self.from_global_keys.get(&key_data).copied()
        } else {
            None
        }
    }

    pub(super) fn insert(&mut self, element_id: ElementId, key: Key) {
        self.element_keys.insert(element_id, key);

        if let Key::Global(key_data) = key {
            self.from_global_keys.insert(key_data, element_id);
        }
    }

    pub(super) fn remove(&mut self, element_id: ElementId) {
        if let Some(Key::Global(key_data)) = self.element_keys.remove(element_id) {
            self.from_global_keys.remove(&key_data);
        }
    }
}
