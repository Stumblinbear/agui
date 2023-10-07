use rustc_hash::FxHashSet;

use crate::{
    element::{Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    render::manager::RenderViewManager,
    util::tree::Tree,
    widget::{ContextElement, ContextMarkDirty},
};

pub struct ElementUnmountContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) element_id: ElementId,
}

impl<'ctx> ContextPlugins<'ctx> for ElementUnmountContext<'ctx> {
    fn get_plugins(&self) -> &Plugins<'ctx> {
        &self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for ElementUnmountContext<'ctx> {
    fn get_plugins_mut(&mut self) -> &mut Plugins<'ctx> {
        &mut self.plugins
    }
}

impl ContextElement for ElementUnmountContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextMarkDirty for ElementUnmountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}
