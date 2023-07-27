use crate::element::ElementId;

#[derive(Default)]
pub struct InheritanceNode {
    // The inheritance scope that this node belongs to.
    scope_id: Option<ElementId>,
}

impl InheritanceNode {
    pub fn new(scope_id: Option<ElementId>) -> Self {
        Self { scope_id }
    }

    pub fn get_scope(&self) -> Option<ElementId> {
        self.scope_id
    }
}
