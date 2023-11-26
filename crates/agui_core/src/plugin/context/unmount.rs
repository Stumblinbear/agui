use crate::{
    element::{ContextElement, ContextElements, Element, ElementId},
    engine::Dirty,
    render::RenderObjectId,
    util::tree::Tree,
};

pub struct PluginElementUnmountContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub needs_build: &'ctx mut Dirty<ElementId>,
    pub needs_layout: &'ctx mut Dirty<RenderObjectId>,
    pub needs_paint: &'ctx mut Dirty<RenderObjectId>,

    pub element_id: &'ctx ElementId,
    pub element: &'ctx Element,
}

impl ContextElements for PluginElementUnmountContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for PluginElementUnmountContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}
