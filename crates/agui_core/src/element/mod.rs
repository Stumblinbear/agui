mod any_element;
mod node;
mod shared;

pub use any_element::*;
pub use node::*;
pub use shared::*;

/// A persistent node in the element tree.
pub trait Element {}

// The unit type is the leaf element: no children, no state.
impl Element for () {}
