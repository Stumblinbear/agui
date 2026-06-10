mod any_element;
mod boundary;
pub mod node;
mod routing_id;
mod shared;

pub use any_element::*;
pub use boundary::*;
pub use routing_id::*;
pub use shared::*;

use std::marker::PhantomData;

use crate::{
    context::Dispatch,
    diagnostics::{Diagnostics, DiagnosticsNode},
};

/// A persistent node in the element tree, holding its widget's state and materialized children.
pub trait Element {
    /// The render object this element owns, threaded through [`dispatch`](Self::dispatch).
    type Render: ?Sized;

    /// Routes `action` along `path` to the destination element, relative to this one, threading
    /// `render` so the destination can reconcile it.
    fn dispatch(&mut self, render: &mut Self::Render, path: &[RoutingId], action: Dispatch) {
        debug_assert!(path.is_empty(), "leaf element has nothing to route to");
        let _ = (render, action);
    }

    /// Captures this element's subtree as a diagnostics snapshot.
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

impl Element for () {
    type Render = ();
}

/// A leaf element with no children, naming the render object its widget produces.
pub struct LeafElement<R: ?Sized>(PhantomData<fn() -> R>);

impl<R: ?Sized> LeafElement<R> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<R: ?Sized> Default for LeafElement<R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: 'static> Element for LeafElement<R> {
    type Render = R;

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}
