use crate::{
    callback::{CallbackQueue, ContextCallbackQueue},
    element::{Element, ElementId},
    engine::Dirty,
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
};

use super::{ContextElement, ContextElements};

pub struct ElementBuildContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,
    pub callback_queue: &'ctx CallbackQueue,

    pub needs_build: &'ctx mut Dirty<ElementId>,

    pub element_id: &'ctx ElementId,
}

impl<'ctx> ContextPlugins<'ctx> for ElementBuildContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for ElementBuildContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextElements for ElementBuildContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementBuildContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextCallbackQueue for ElementBuildContext<'_> {
    fn callback_queue(&self) -> &CallbackQueue {
        self.callback_queue
    }
}
