use crate::{
    element::{ContextElement, Element, ElementId},
    unit::Offset,
    util::tree::Tree,
    widget::{IterChildrenLayout, IterChildrenLayoutMut},
};

pub struct WidgetLayoutContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
    pub(crate) offsets: &'ctx mut [Offset],
}

impl ContextElement for WidgetLayoutContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl WidgetLayoutContext<'_> {
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
