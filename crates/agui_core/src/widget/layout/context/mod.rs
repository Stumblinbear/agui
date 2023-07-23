mod intrinsic_size;
mod layout;

pub use intrinsic_size::*;
pub use layout::*;

use super::{IterChildren, IterChildrenMut};

pub trait ContextWidgetLayout {
    fn has_children(&self) -> bool;

    fn child_count(&self) -> usize;

    fn iter_children(&self) -> IterChildren;
}

pub trait ContextWidgetLayoutMut: ContextWidgetLayout {
    fn iter_children_mut(&mut self) -> IterChildrenMut;
}
