use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{
        ContextDirtyRenderObject, ContextElement, ContextElements, ContextRenderObject, Element,
        ElementId,
    },
    engine::Dirty,
    render::RenderObjectId,
    util::tree::Tree,
};

pub struct UpdatePluginRenderObjectContext<'ctx, 'element> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub callback_queue: &'ctx CallbackQueue,

    pub needs_build: &'ctx mut Dirty<ElementId>,
    pub needs_layout: &'ctx mut Dirty<RenderObjectId>,
    pub needs_paint: &'ctx mut Dirty<RenderObjectId>,

    pub element_id: &'ctx ElementId,

    pub render_object_id: &'element RenderObjectId,
}

impl ContextElements for UpdatePluginRenderObjectContext<'_, '_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for UpdatePluginRenderObjectContext<'_, '_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextRenderObject for UpdatePluginRenderObjectContext<'_, '_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl ContextDirtyRenderObject for UpdatePluginRenderObjectContext<'_, '_> {
    fn mark_needs_layout(&mut self) {
        self.needs_layout.insert(*self.render_object_id)
    }

    fn mark_needs_paint(&mut self) {
        self.needs_paint.insert(*self.render_object_id)
    }
}

impl ContextCallbackQueue for UpdatePluginRenderObjectContext<'_, '_> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.callback_queue
    }
}
