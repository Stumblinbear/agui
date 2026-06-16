//! Boundary tracking: a registry that owns boundary cells by key, and dirty queues of keys awaiting work.

use slotmap::SlotMap;

/// Owns boundary cells by value, resolving a key back to its cell.
///
/// A handle holds the key for its boundary and resolves through here. Presence is liveness: dropping a
/// boundary [`remove`](Self::remove)s its cell, so its key then resolves to `None`, and a key whose
/// boundary is gone is simply skipped wherever it surfaces.
pub struct BoundaryRegistry<K, V>
where
    K: slotmap::Key,
{
    cells: SlotMap<K, V>,
}

impl<K, V> Default for BoundaryRegistry<K, V>
where
    K: slotmap::Key,
{
    fn default() -> Self {
        Self {
            cells: SlotMap::with_key(),
        }
    }
}

impl<K, V> BoundaryRegistry<K, V>
where
    K: slotmap::Key,
{
    /// Stores `cell`, returning the key that resolves back to it.
    pub fn insert(&mut self, cell: V) -> K {
        self.cells.insert(cell)
    }

    /// Removes the cell registered under `key`, returning it, or `None` if no boundary is registered
    /// under it.
    pub fn remove(&mut self, key: K) -> Option<V> {
        self.cells.remove(key)
    }

    /// Resolves `key` to its cell, or `None` if no boundary is registered under it.
    pub fn get(&self, key: K) -> Option<&V> {
        self.cells.get(key)
    }

    /// Resolves `key` to its cell for mutation, or `None` if no boundary is registered under it.
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        self.cells.get_mut(key)
    }
}

/// A queue of boundary keys awaiting work.
///
/// [`mark`](Self::mark) pushes a key; [`drain_into`](Self::drain_into) hands them all to a caller-owned
/// buffer by swapping the two allocations, so a queue paired with one reused buffer never reallocates
/// once both have grown to the high-water mark. The queue does not dedup or track liveness: a consumer
/// guards a double-mark with a per-cell flag before it calls `mark`, and on drain resolves each key
/// through a [`Registry`] and skips the stale ones. A key whose boundary was dropped resolves to `None`;
/// a key whose work was already done in place has its per-cell flag clear.
pub struct DirtyQueue<K> {
    keys: Vec<K>,
}

impl<K> Default for DirtyQueue<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> DirtyQueue<K> {
    /// A queue with no keys awaiting work.
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    /// Queues `key` for work.
    pub fn mark(&mut self, key: K) {
        self.keys.push(key);
    }

    /// Whether no key is awaiting work.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Moves the queued keys into `out` and leaves the queue empty, reusing both allocations: `out`'s
    /// buffer becomes the queue's, and the queue's becomes `out`'s. Pairing a queue with one reused `out`
    /// buffer means neither reallocates once both have grown to the high-water mark.
    pub fn drain_into(&mut self, out: &mut Vec<K>) {
        out.clear();
        std::mem::swap(&mut self.keys, out);
    }

    /// Discards every queued key without processing it.
    pub fn clear(&mut self) {
        self.keys.clear();
    }
}
