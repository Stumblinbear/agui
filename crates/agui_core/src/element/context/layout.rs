use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
    widget::ContextElement,
};

pub struct ElementLayoutContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
}

impl ContextElement for ElementLayoutContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}
