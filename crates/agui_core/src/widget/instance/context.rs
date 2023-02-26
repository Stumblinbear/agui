use fnv::FnvHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    inheritance::Inheritance,
    unit::Point,
    util::tree::Tree,
};

pub struct WidgetBuildContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) inheritance: &'ctx mut Inheritance,
}

pub struct WidgetCallbackContext<'ctx> {
    pub(crate) element_tree: &'ctx Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,

    pub(crate) element_id: ElementId,
}

pub struct WidgetIntrinsicSizeContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
}

pub struct WidgetLayoutContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,

    pub(crate) element_id: ElementId,

    pub(crate) children: &'ctx [ElementId],
    pub(crate) offsets: &'ctx mut [Option<Point>],
}
