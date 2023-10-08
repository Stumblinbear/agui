use rustc_hash::FxHashSet;

use crate::element::ElementId;

pub struct DirtyElements {
    inner: FxHashSet<ElementId>,
}

impl DirtyElements {
    pub(super) fn new() -> Self {
        Self {
            inner: FxHashSet::default(),
        }
    }

    /// Check if any elements have been marked as dirty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Check if a given element has been marked as dirty.
    pub fn is_dirty(&self, element_id: ElementId) -> bool {
        self.inner.contains(&element_id)
    }

    /// Mark an element as dirty, causing it to be rebuilt at the next opportunity.
    pub fn insert(&mut self, element_id: ElementId) {
        self.inner.insert(element_id);
    }

    pub(super) fn drain(&mut self) -> impl Iterator<Item = ElementId> + '_ {
        self.inner.drain()
    }
}
