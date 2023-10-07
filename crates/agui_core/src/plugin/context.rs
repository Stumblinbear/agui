use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    render::manager::RenderViewManager,
    util::tree::Tree,
};

pub struct PluginMountContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) parent_element_id: Option<ElementId>,
    pub(crate) element_id: ElementId,
}

pub struct PluginUnmountContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

pub struct PluginBuildContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,
}
