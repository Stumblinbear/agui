use crate::element::ElementId;

use super::ContextElement;

pub struct RenderObjectCreateContext<'ctx> {
    pub element_id: &'ctx ElementId,
}

impl ContextElement for RenderObjectCreateContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}
