use crate::{
    element::{Element, ElementId},
    engine::elements::scheduler::ElementScheduler,
    inheritance::InheritanceManager,
    util::tree::Tree,
};

pub struct ElementTreeContext<'ctx> {
    pub scheduler: ElementScheduler<'ctx>,

    pub tree: &'ctx Tree<ElementId, Element>,
    pub inheritance: &'ctx mut InheritanceManager,

    pub element_id: &'ctx ElementId,
}

pub struct ElementTreeMountContext<'ctx> {
    pub tree: &'ctx Tree<ElementId, Element>,

    pub parent_element_id: &'ctx Option<ElementId>,
    pub element_id: &'ctx ElementId,
}
