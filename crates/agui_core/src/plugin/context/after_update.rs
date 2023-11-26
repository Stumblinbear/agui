use crate::{
    element::{ContextElements, ContextRenderObjects, Element, ElementId},
    render::{RenderObject, RenderObjectId},
    util::tree::Tree,
};

pub struct PluginAfterUpdateContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,
}

impl ContextElements for PluginAfterUpdateContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextRenderObjects for PluginAfterUpdateContext<'_> {
    fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject> {
        self.render_object_tree
    }
}
