use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{ContextElement, ContextMarkDirty, Element, ElementId, ContextElements},
    engine::DirtyElements,
    util::tree::Tree,
};

pub struct PluginElementBuildContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub dirty: &'ctx mut DirtyElements,

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

impl ContextMarkDirty for PluginElementBuildContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl ContextCallbackQueue for PluginElementBuildContext<'_> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.callback_queue
    }
}
