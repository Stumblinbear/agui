use rustc_hash::FxHashSet;

use crate::{
    element::{Element, ElementId},
    render::manager::RenderViewManager,
    util::tree::Tree,
    widget::ContextWidget,
};

pub struct PluginUnmountContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

impl ContextWidget for PluginUnmountContext<'_> {
    fn get_elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }

    fn get_element_id(&self) -> ElementId {
        self.element_id
    }
}
