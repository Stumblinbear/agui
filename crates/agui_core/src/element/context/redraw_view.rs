use crate::{
    element::{Element, ElementId},
    render::{RenderObject, RenderObjectId},
    util::tree::Tree,
};

use super::{ContextElement, ContextElements, ContextRenderObject};

pub struct RedrawViewContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub element_id: &'ctx ElementId,
    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextElements for RedrawViewContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for RedrawViewContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextRenderObject for RedrawViewContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}
