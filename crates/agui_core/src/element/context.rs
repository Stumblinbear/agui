use fnv::FnvHashSet;

use crate::{
    callback::CallbackQueue,
    element::{Element, ElementId},
    inheritance::Inheritance,
    util::tree::Tree,
};

pub struct ElementContext<'ctx> {
    pub(crate) element_tree: &'ctx mut Tree<ElementId, Element>,
    pub(crate) dirty: &'ctx mut FnvHashSet<ElementId>,
    pub(crate) callback_queue: &'ctx CallbackQueue,

    pub(crate) element_id: ElementId,

    pub(crate) inheritance: &'ctx mut Inheritance,
}
