//! The tree-based build phase. Every element implements [`Element`] and lives in
//! an [`agui_core::tree::Tree`] keyed by [`Build`]; this module defines that dispatch binding and the glue
//! the tree stores. The glue unwraps [`Operation`] onto the matching element method, and the core tree names
//! no node trait, so `Element` is wholly `agui`'s. The per-hook contexts live in [`crate::context`].

use std::ptr::NonNull;

use agui_core::dirty::Dirty;
use agui_core::tree::{NodeDispatch, NodeHandle, Tree};
use rustc_hash::FxHashSet;

use crate::context::{MessageCtx, UpdateCtx};
use crate::element::Element;

/// The element tree's dispatch binding. [`Tree::dispatch`](agui_core::tree::Tree::dispatch) delivers an
/// [`Operation`] to a node by handle.
pub struct Build;

impl NodeDispatch for Build {
    type Operation<'a> = Operation<'a>;
}

/// The dispatch fn handed to the tree for a `C`: casts the node pointer back to `C` and unwraps the
/// operation onto the matching method.
pub(crate) unsafe fn run<C: Element>(data: NonNull<()>, operation: Operation<'_>) {
    // SAFETY: only ever stored against a `data` that points at a live `C`.
    let node = unsafe { &mut *data.cast::<C>().as_ptr() };
    match operation {
        Operation::Rebuild(mut ctx) => node.rebuild(&mut ctx),
        Operation::DependencyChanged(mut ctx) => node.dependency_changed(&mut ctx),
        Operation::Message(mut ctx) => node.message(&mut ctx),
    }
}

/// A build-phase operation delivered to one element by handle. Only `Rebuild` and `DependencyChanged` carry
/// a cursor, so a message cannot restructure the tree. Layout and paint are driven from a boundary registry,
/// not dispatched to elements.
pub enum Operation<'a> {
    /// Re-run the element's build.
    Rebuild(UpdateCtx<'a>),
    /// A provided value the element depends on changed.
    DependencyChanged(UpdateCtx<'a>),
    /// Deliver a message; the element may mutate state and request a rebuild.
    Message(MessageCtx<'a>),
}

/// The build-phase work queue: the handles waiting to rebuild, plus which of them changed a provided value
/// and so also run their dependency-change hook before rebuilding. Wraps the core [`Dirty`] set with that
/// dependency-change distinction; held by the driver and threaded to elements through their contexts.
#[derive(Default)]
pub struct BuildQueue {
    dirty: Dirty,
    dependency_changed: FxHashSet<NodeHandle>,
}

impl BuildQueue {
    pub fn new() -> Self {
        Self {
            dirty: Dirty::new(),
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
