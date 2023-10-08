use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    engine::DirtyElements,
    util::tree::Tree,
};

pub struct PluginBuildContext<'ctx> {
    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub dirty: &'ctx mut DirtyElements,

    pub element_id: &'ctx ElementId,

    pub callback_queue: &'ctx CallbackQueue,
}

impl ContextElement for PluginBuildContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextMarkDirty for PluginBuildContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl ContextCallbackQueue for PluginBuildContext<'_> {
    fn get_callback_queue(&self) -> &CallbackQueue {
        self.callback_queue
    }
}
