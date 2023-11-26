use crate::{
    element::{ContextElement, Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    util::tree::Tree,
};

use super::ContextElements;

pub struct ElementMountContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub parent_element_id: Option<&'ctx ElementId>,
    pub element_id: &'ctx ElementId,
}

impl<'ctx> ContextPlugins<'ctx> for ElementMountContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for ElementMountContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextElements for ElementMountContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for ElementMountContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ElementMountContext<'_> {
    pub fn parent_element_id(&self) -> Option<ElementId> {
        self.parent_element_id.copied()
    }
}
