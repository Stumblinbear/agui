use rustc_hash::FxHashSet;

use crate::dirty_list::DirtyList;
use crate::tree::{NodeDispatch, NodeHandle, Tree};

/// The handles waiting to rebuild, and which of them also changed a provided value and so run their
/// dependency-change hook before their rebuild.
#[derive(Default)]
pub struct BuildQueue {
    dirty: DirtyList,
    dependency_changed: FxHashSet<NodeHandle>,
}

impl BuildQueue {
    pub fn new() -> Self {
        Self {
            dirty: DirtyList::new(),
            dependency_changed: FxHashSet::default(),
        }
    }

    /// Whether nothing is queued.
    pub fn is_empty(&self) -> bool {
        self.dirty.is_empty()
    }

    /// Queues `handle` to rebuild on the next flush.
    pub fn mark_rebuild(&mut self, handle: NodeHandle) {
        self.dirty.mark(handle);
    }

    /// Queues `handle` to rebuild on the next flush, running its dependency-change hook first.
    pub fn mark_dependency_changed(&mut self, handle: NodeHandle) {
        self.dirty.mark(handle);
        self.dependency_changed.insert(handle);
    }

    /// Removes and returns the shallowest queued handle whose node is still in `tree`, paired with whether it
    /// is a dependency change. Returns `None` once nothing live remains.
    pub fn take_shallowest<R, N: NodeDispatch>(
        &mut self,
        tree: &Tree<R, N>,
    ) -> Option<(NodeHandle, bool)> {
        let Some(handle) = self.dirty.take_shallowest(tree) else {
            // Any dependency marks left over name handles dropped before they drained; clear them.
            self.dependency_changed.clear();
            return None;
        };

        let is_dependency_change = self.dependency_changed.remove(&handle);
        Some((handle, is_dependency_change))
    }
}
