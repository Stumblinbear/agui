mod build;
mod callback;
mod hit_test;
mod intrinsic_size;
mod layout;
mod mount;
mod unmount;

pub use build::*;
pub use callback::*;
pub use hit_test::*;
pub use intrinsic_size::*;
pub use layout::*;
pub use mount::*;
pub use unmount::*;

use crate::util::tree::Tree;

use super::{Element, ElementId};

pub trait ContextElement {
    fn get_elements(&self) -> &Tree<ElementId, Element>;

    fn get_element_id(&self) -> ElementId;
}

pub trait ContextMarkDirty {
    fn mark_dirty(&mut self, element_id: ElementId);
}
