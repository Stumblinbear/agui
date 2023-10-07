use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    plugin::Plugins,
    render::manager::RenderViewManager,
    unit::HitTestResult,
    util::tree::Tree,
    widget::ContextMarkDirty,
};

pub struct ElementMountContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) parent_element_id: Option<ElementId>,
    pub(crate) element_id: ElementId,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
}

impl ContextMarkDirty for ElementMountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

pub struct ElementUnmountContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) render_view_manager: &'ctx mut RenderViewManager,

    pub(crate) element_id: ElementId,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
}

impl ContextMarkDirty for ElementUnmountContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

pub struct ElementBuildContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) element_id: ElementId,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,

    pub(crate) callback_queue: &'ctx CallbackQueue,
}

impl ContextMarkDirty for ElementBuildContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
}

pub struct ElementCallbackContext<'ctx> {
    pub(crate) plugins: Plugins<'ctx>,

    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) dirty: &'ctx mut FxHashSet<ElementId>,
}

impl ContextMarkDirty for ElementCallbackContext<'_> {
    fn mark_dirty(&mut self, element_id: ElementId) {
        self.dirty.insert(element_id);
    }
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
