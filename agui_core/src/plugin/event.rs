use std::any::TypeId;

use crate::{widget::WidgetId, unit::Rect};

#[derive(Copy, Clone, Debug)]
#[non_exhaustive]
pub enum WidgetEvent {
    Added {
        type_id: TypeId,
        widget_id: WidgetId,
    },
    
    Layout {
        type_id: TypeId,
        widget_id: WidgetId,
        rect: Rect,
    },

    Removed {
        type_id: TypeId,
        widget_id: WidgetId,
    },

    Updated
}
