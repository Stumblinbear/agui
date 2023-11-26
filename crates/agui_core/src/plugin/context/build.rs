use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{ContextElement, ContextElements, Element, ElementId},
    engine::Dirty,
    render::RenderObjectId,
    util::tree::Tree,
};

pub struct PluginElementBuildContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub needs_build: &'ctx mut Dirty<ElementId>,
    pub needs_layout: &'ctx mut Dirty<RenderObjectId>,
    pub needs_paint: &'ctx mut Dirty<RenderObjectId>,

    pub element_id: &'ctx ElementId,
    pub element: &'ctx Element,

    pub callback_queue: &'ctx CallbackQueue,
}

impl ContextElements for PluginElementBuildContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for PluginElementBuildContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextCallbackQueue for PluginElementBuildContext<'_> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.callback_queue
    }
}
