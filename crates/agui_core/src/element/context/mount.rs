use crate::{
    element::{ContextElement, Element, ElementId},
    inheritance::InheritanceManager,
    util::tree::Tree,
};

use super::ContextElements;

pub struct ElementMountContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance: &'ctx mut InheritanceManager,

    pub parent_element_id: &'ctx Option<ElementId>,
    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementMountContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementMountContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ElementMountContext<'_> {
    pub fn parent_element_id(&self) -> Option<ElementId> {
        *self.parent_element_id
    }
}
