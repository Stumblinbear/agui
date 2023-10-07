use crate::{
    element::{ContextElement, Element, ElementId},
    unit::HitTestResult,
    util::tree::Tree,
};

pub struct ElementHitTestContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) result: &'ctx mut HitTestResult,
}

impl ContextElement for ElementHitTestContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}
