use crate::{element::ElementId, render::RenderViewId};

/// Used to indicate a change to elements in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ElementEvent {
    /// A element has been spawned.
    Spawned {
        parent_id: Option<ElementId>,
        element_id: ElementId,
    },

    /// A element has been rebuilt.
    Rebuilt { element_id: ElementId },

    /// A element has been reparented.
    Reparent {
        parent_id: Option<ElementId>,
        element_id: ElementId,
    },

    /// A element has been destroyed.
    Destroyed { element_id: ElementId },

    /// A element needs to be redrawn. This will occur the first time a element is drawn and for subsequent changes.
    Draw {
        render_view_id: RenderViewId,
        element_id: ElementId,
    },
}

impl ElementEvent {
    pub fn element_id(&self) -> &ElementId {
        match self {
            ElementEvent::Spawned { element_id, .. }
            | ElementEvent::Rebuilt { element_id, .. }
            | ElementEvent::Reparent { element_id, .. }
            | ElementEvent::Destroyed { element_id, .. }
            | ElementEvent::Draw { element_id, .. } => element_id,
        }
    }
}
