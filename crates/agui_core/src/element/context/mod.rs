use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
};

mod build;
mod callback;
mod mount;
mod unmount;

pub use build::*;
pub use callback::*;
pub use mount::*;
pub use unmount::*;

pub trait ContextElements {
    fn elements(&self) -> &Tree<ElementId, Element>;
}

pub trait ContextElement {
    fn element_id(&self) -> ElementId;
}

pub trait ContextMarkDirty {
    fn mark_dirty(&mut self, element_id: ElementId);
}

pub struct ElementContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

pub struct ElementContextMut<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementContextMut<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementContextMut<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}
