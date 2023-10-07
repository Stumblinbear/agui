use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
    widget::ContextWidget,
};

pub struct WidgetBuildContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,
}

impl ContextWidget for WidgetBuildContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl<'ctx> ContextPlugins<'ctx> for WidgetBuildContext<'ctx> {
    fn get_plugins(&self) -> &Plugins<'ctx> {
        &self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for WidgetBuildContext<'ctx> {
    fn get_plugins_mut(&mut self) -> &mut Plugins<'ctx> {
        &mut self.plugins
    }
}

impl WidgetBuildContext<'_> {
    pub fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
