mod any_element;
mod boundary;
pub mod node;
mod routing_id;
mod shared;

pub use any_element::*;
pub use boundary::*;
pub use routing_id::*;
pub use shared::*;

/// A persistent node in the element tree.
pub trait Element {}

impl Element for () {}
