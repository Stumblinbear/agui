use crate::{
    element::{ContextElement, Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
};

use super::ContextElements;

pub struct ElementUnmountContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub element_id: &'ctx ElementId,
}

impl<'ctx> ContextPlugins<'ctx> for ElementUnmountContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for ElementUnmountContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextElements for ElementUnmountContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementUnmountContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}
