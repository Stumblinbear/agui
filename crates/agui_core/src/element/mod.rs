mod any_element;
mod boundary;
pub mod node;
mod routing_id;
mod shared;

pub use any_element::*;
pub use boundary::*;
pub use routing_id::*;
pub use shared::*;

use crate::context::Dispatch;

/// A persistent node in the element tree, holding its widget's state and materialized children.
pub trait Element {
    /// Routes `action` along `path` to the destination element, relative to this one.
    fn dispatch(&mut self, path: &[RoutingId], action: Dispatch) {
        debug_assert!(path.is_empty(), "leaf element has nothing to route to");
        let _ = action;
    }
}

impl Element for () {}
