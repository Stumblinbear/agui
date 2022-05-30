use crate::widget::WidgetId;

/// Used to indicate a change to widgets in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WidgetEvent {
    /// A widget has been spawned.
    Spawned {
        parent_id: Option<WidgetId>,
        widget_id: WidgetId,
    },

    /// A widget has been rebuilt.
    Rebuilt { widget_id: WidgetId },

    /// A widget has been reparented.
    Reparent {
        parent_id: Option<WidgetId>,
        widget_id: WidgetId,
    },

    /// A widget has changed in the layout.
    Layout { widget_id: WidgetId },

    /// A widget has been destroyed.
    Destroyed { widget_id: WidgetId },
}

impl WidgetEvent {
    pub fn widget_id(&self) -> &WidgetId {
        match self {
            WidgetEvent::Spawned { widget_id, .. }
            | WidgetEvent::Rebuilt { widget_id, .. }
            | WidgetEvent::Reparent { widget_id, .. }
            | WidgetEvent::Layout { widget_id, .. }
            | WidgetEvent::Destroyed { widget_id, .. } => widget_id,
        }
    }
}
