use fnv::FnvHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    util::tree::Tree,
};

pub struct ElementMountContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
}

pub struct ElementUnmountContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,
}

pub struct ElementBuildContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,
}

pub struct ElementCallbackContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

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
