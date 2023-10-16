use crate::{element::ElementId, listenable::Event};

/// A element has been spawned.
#[non_exhaustive]
pub struct ElementSpawnedEvent {
    pub parent_id: Option<ElementId>,
    pub element_id: ElementId,
}

impl Event for ElementSpawnedEvent {}

/// A element has been rebuilt.
#[non_exhaustive]
pub struct ElementRebuiltEvent {
    pub element_id: ElementId,
}

impl Event for ElementRebuiltEvent {}

/// A element has been reparented.
#[non_exhaustive]
pub struct ElementReparentEvent {
    pub parent_id: Option<ElementId>,
    pub element_id: ElementId,
}

impl Event for ElementReparentEvent {}

/// A element has been destroyed.
#[non_exhaustive]
pub struct ElementDestroyedEvent {
    pub element_id: ElementId,
}

impl Event for ElementDestroyedEvent {}
