use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
};

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

pub trait ContextElement {
    fn get_elements(&self) -> &Tree<ElementId, Element>;

    fn get_element_id(&self) -> ElementId;
}

pub trait ContextMarkDirty {
    fn mark_dirty(&mut self, element_id: ElementId);
}

pub struct ElementContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,
}

impl ContextElement for ElementContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

pub struct ElementContextMut<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,
}

impl ContextElement for ElementContextMut<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}
