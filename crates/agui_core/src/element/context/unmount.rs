use crate::{
    element::{ContextElement, Element, ElementId},
    inheritance::InheritanceManager,
    util::tree::Tree,
};

use super::ContextElements;

pub struct ElementUnmountContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance: &'ctx mut InheritanceManager,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementUnmountContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementUnmountContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}
