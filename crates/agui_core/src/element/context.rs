use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    plugin::Plugins,
    render::manager::RenderViewManager,
    unit::HitTestResult,
    util::tree::Tree,
};

pub struct ElementMountContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) parent_element_id: Option<ElementId>,
    pub(crate) element_id: ElementId,
}

pub struct ElementUnmountContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

pub struct ElementBuildContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,
}

pub struct ElementCallbackContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

pub struct ElementIntrinsicSizeContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
}

pub struct ElementLayoutContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
}

pub struct ElementHitTestContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) result: &'ctx mut HitTestResult,
}
