use crate::widget::WidgetId;

use super::LayerId;

#[derive(Debug)]
pub enum RenderError {
    MissingWidget {
        widget_id: WidgetId,
    },

    MissingLayer {
        layer_id: LayerId,
    },

    NoLayerTarget {
        parent_id: WidgetId,
        widget_id: WidgetId,
    },
}
