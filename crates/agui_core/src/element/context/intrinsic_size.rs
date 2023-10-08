use crate::{
    element::{ContextElement, Element, ElementId, IterChildrenLayout},
    util::tree::Tree,
};

pub struct ElementIntrinsicSizeContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,

    pub children: &'ctx [ElementId],
}

impl ContextElement for ElementIntrinsicSizeContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl<'ctx> ElementIntrinsicSizeContext<'ctx> {
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn iter_children(&self) -> IterChildrenLayout {
        IterChildrenLayout::new(self.element_tree, self.children)
    }
}
