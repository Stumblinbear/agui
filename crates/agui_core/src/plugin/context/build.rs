use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{ContextElement, ContextMarkDirty, Element, ElementId},
    engine::DirtyElements,
    plugin::Plugins,
    util::tree::Tree,
};

pub struct PluginElementBuildContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub dirty: &'ctx mut DirtyElements,

    pub element_id: &'ctx ElementId,

    pub callback_queue: &'ctx CallbackQueue,
}

impl ContextElement for PluginElementBuildContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextMarkDirty for PluginElementBuildContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

impl ContextCallbackQueue for PluginElementBuildContext<'_> {
    fn get_callback_queue(&self) -> &CallbackQueue {
        self.callback_queue
    }
}
