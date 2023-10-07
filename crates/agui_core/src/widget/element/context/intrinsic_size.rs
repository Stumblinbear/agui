use crate::{
    element::{ContextElement, Element, ElementId},
    util::tree::Tree,
    widget::IterChildrenLayout,
};

pub struct WidgetIntrinsicSizeContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
}

impl ContextElement for WidgetIntrinsicSizeContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx> WidgetIntrinsicSizeContext<'ctx> {
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
