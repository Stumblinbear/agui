use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
};

use super::{ContextElement, ContextMarkDirty};

pub struct ElementBuildContext<'ctx> {
    pub(crate) plugins: &'ctx mut Plugins,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,

    pub(crate) callback_queue: &'ctx CallbackQueue,
}

impl<'ctx> ContextPlugins<'ctx> for ElementBuildContext<'ctx> {
    fn get_plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for ElementBuildContext<'ctx> {
    fn get_plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextElement for ElementBuildContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextMarkDirty for ElementBuildContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
