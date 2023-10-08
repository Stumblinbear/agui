use crate::{
    element::{ContextElement, Element, ElementId},
    unit::Offset,
    util::tree::Tree,
};

mod iter;

pub use iter::*;

pub struct ElementLayoutContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,

    pub children: &'ctx [ElementId],
    pub offsets: &'ctx mut [Offset],
}

impl ContextElement for ElementLayoutContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ElementLayoutContext<'_> {
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn iter_children(&self) -> IterChildrenLayout {
        IterChildrenLayout::new(self.element_tree, self.children)
    }

    pub fn iter_children_mut(&mut self) -> IterChildrenLayoutMut {
        IterChildrenLayoutMut::new(self.element_tree, self.children, self.offsets)
    }
}
