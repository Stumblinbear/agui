use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{ContextElement, ContextElements, ContextRenderObject, Element, ElementId},
    engine::Dirty,
    render::RenderObjectId,
    util::tree::Tree,
};

pub struct PluginCreateRenderObjectContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub needs_build: &'ctx mut Dirty<ElementId>,
    pub needs_layout: &'ctx mut Dirty<RenderObjectId>,
    pub needs_paint: &'ctx mut Dirty<RenderObjectId>,

    pub element_id: &'ctx ElementId,
    pub element: &'ctx Element,

    pub callback_queue: &'ctx CallbackQueue,

    pub render_object_id: &'ctx RenderObjectId,
}

impl ContextElements for PluginCreateRenderObjectContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for PluginCreateRenderObjectContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextRenderObject for PluginCreateRenderObjectContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl ContextCallbackQueue for PluginCreateRenderObjectContext<'_> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.callback_queue
    }
}
