use std::any::TypeId;

use crate::widget::WidgetId;

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

    /// A widget has been destroyed.
    Destroyed {
        type_id: TypeId,
        widget_id: WidgetId,
    },
}
