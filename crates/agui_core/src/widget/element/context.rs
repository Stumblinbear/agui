use fnv::FnvHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    gestures::hit_test::HitTestEntry,
    inheritance::manager::InheritanceManager,
    render::{manager::RenderContextManager, RenderContextId},
    unit::{Offset, Size},
    util::tree::Tree,
};

pub struct WidgetMountContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance_manager: &'ctx mut InheritanceManager,
    pub(crate) render_context_manager: &'ctx mut RenderContextManager,

    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) parent_element_id: Option<ElementId>,
    pub(crate) element_id: ElementId,
}

pub struct WidgetUnmountContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance_manager: &'ctx mut InheritanceManager,

    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

pub struct WidgetBuildContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance_manager: &'ctx mut InheritanceManager,

    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,
}

pub struct WidgetCallbackContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) inheritance_manager: &'ctx InheritanceManager,

    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

pub struct WidgetIntrinsicSizeContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
}

pub struct WidgetLayoutContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
    pub(crate) offsets: &'ctx mut [Offset],
}

pub struct WidgetHitTestContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,

    pub(crate) path: &'ctx mut Vec<HitTestEntry>,

    pub(crate) element_id: ElementId,

    pub(crate) size: &'ctx Size,

    pub(crate) children: &'ctx [ElementId],
}
