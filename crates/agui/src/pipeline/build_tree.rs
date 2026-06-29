//! The tree-based build phase. Every element implements [`Element`] and lives in
//! an [`agui_core::tree::Tree`] keyed by [`Build`]; this module defines that dispatch binding and the glue
//! the tree stores. The glue unwraps [`Operation`] onto the matching element method, and the core tree names
//! no node trait, so `Element` is wholly `agui`'s. The per-hook contexts live in [`crate::context`].

use std::ptr::NonNull;

use agui_core::tree::NodeDispatch;

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
