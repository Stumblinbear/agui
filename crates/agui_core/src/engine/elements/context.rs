use crate::{
    element::{Element, ElementId},
    engine::elements::scheduler::ElementScheduler,
    inheritance::InheritanceManager,
    util::tree::Tree,
};

pub struct ElementTreeContext<'ctx> {
    pub tree: &'ctx Tree<ElementId, Element>,

    pub scheduler: ElementScheduler<'ctx>,

    pub element_id: &'ctx ElementId,

    pub inheritance: &'ctx mut InheritanceManager,
}
