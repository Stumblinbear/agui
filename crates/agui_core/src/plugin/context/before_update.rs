use crate::{
    element::{ContextElements, Element, ElementId},
    util::tree::Tree,
};

pub struct PluginBeforeUpdateContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
}

impl ContextElements for PluginBeforeUpdateContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}
