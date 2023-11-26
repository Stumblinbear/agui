use std::any::TypeId;

use agui_core::{element::ElementId, util::map::TypeIdSet};

#[derive(PartialEq, Debug, Default)]
pub struct InheritanceNode {
    // The inheritance scope that this node belongs to.
    scope_id: Option<ElementId>,

    /// Tracks which types this node depends on.
    dependencies: TypeIdSet,
}

impl InheritanceNode {
    pub fn new(scope_id: Option<ElementId>) -> Self {
        Self {
            scope_id,

            dependencies: TypeIdSet::default(),
        }
    }

    pub fn scope(&self) -> Option<ElementId> {
        self.scope_id
    }

    pub fn set_scope(&mut self, scope_id: Option<ElementId>) {
        self.scope_id = scope_id;
    }

    pub fn iter_dependencies(&self) -> impl Iterator<Item = TypeId> + '_ {
        self.dependencies.iter().copied()
    }

    pub fn add_dependency(&mut self, type_id: TypeId) {
        self.dependencies.insert(type_id);
    }

    pub fn remove_dependency(&mut self, type_id: &TypeId) {
        self.dependencies.remove(type_id);
    }
}
