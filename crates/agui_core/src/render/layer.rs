use slotmap::new_key_type;

use crate::widget::WidgetId;

use super::canvas::{command::CanvasCommand, CanvasStyle};

new_key_type! {
    pub struct LayerId;
}

pub struct Layer {
    pub style: CanvasStyle,

    pub widgets: Vec<(WidgetId, LayerWidget)>,
}

pub struct LayerWidget {
    pub widget_id: WidgetId,

    pub canvas: LayerCanvas,
}

pub struct LayerCanvas {
    pub head: Vec<CanvasCommand>,
    pub children: Vec<Layer>,
}
