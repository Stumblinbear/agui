use std::any::TypeId;

use crate::widget::{Widget, WidgetId};

/// Used to indicate a change to widgets in the tree.
#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum WidgetEvent {
    /// A widget has been spawned.
    Spawned {
        type_id: TypeId,
        widget_id: WidgetId,
    },

    /// A widget has changed in the layout.
    Layout {
        type_id: TypeId,
        widget_id: WidgetId,
        layer: u32,
    },

    /// A widget has been rebuilt.
    Rebuilt {
        type_id: TypeId,
        widget_id: WidgetId,
    },

    /// A widget has been destroyed.
    Destroyed {
        type_id: TypeId,
        widget_id: WidgetId,
    },
}

impl WidgetEvent {
    pub fn widget_id(&self) -> &WidgetId {
        match self {
            WidgetEvent::Spawned { widget_id, .. }
            | WidgetEvent::Rebuilt { widget_id, .. }
            | WidgetEvent::Destroyed { widget_id, .. }
            | WidgetEvent::Layout { widget_id, .. } => widget_id,
        }
    }

    pub fn is_widget<W>(&self) -> bool
    where
        W: Widget + 'static,
    {
        match self {
            WidgetEvent::Spawned { type_id, .. }
            | WidgetEvent::Rebuilt { type_id, .. }
            | WidgetEvent::Destroyed { type_id, .. }
            | WidgetEvent::Layout { type_id, .. } => TypeId::of::<W>() == *type_id,
        }
    }
}
