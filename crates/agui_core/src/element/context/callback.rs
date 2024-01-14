use crate::{
    element::{ContextDirtyElement, ContextElement, Element, ElementId},
    engine::Dirty,
    util::tree::Tree,
};

use super::ContextElements;

pub struct ElementCallbackContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub needs_build: &'ctx mut Dirty<ElementId>,

    pub element_id: &'ctx ElementId,
}

impl ContextElements for ElementCallbackContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementCallbackContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextDirtyElement for ElementCallbackContext<'_> {
    fn mark_needs_build(&mut self) {
        self.needs_build.insert(*self.element_id);
    }
}
