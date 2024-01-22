use crate::{
    element::{ContextDirtyElement, ContextElement, ElementId},
    engine::Dirty,
};

pub struct ElementTaskContext {
    pub(crate) element_id: ElementId,

    pub(crate) needs_build: Dirty<ElementId>,
}

impl ContextElement for ElementTaskContext {
    fn element_id(&self) -> ElementId {
        self.element_id
    }
}

impl ContextDirtyElement for ElementTaskContext {
    fn mark_needs_build(&mut self) {
        self.needs_build.insert(self.element_id);
        self.needs_build.notify();
    }
}
