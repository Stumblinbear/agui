use crate::{
    element::{ContextElement, Element, ElementId},
    util::tree::Tree,
};

pub struct ElementIntrinsicSizeContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
}

impl ContextElement for ElementIntrinsicSizeContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}
