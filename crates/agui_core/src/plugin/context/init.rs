use crate::{
    element::{ContextElements, Element, ElementId},
    listenable::EventBus,
    util::tree::Tree,
};

pub struct PluginInitContext<'ctx> {
    pub bus: &'ctx EventBus,

    pub element_tree: &'ctx Tree<ElementId, Element>,
}

impl ContextElements for PluginInitContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}
