use slotmap::new_key_type;

use crate::{canvas::Canvas, unit::Rect, widget::WidgetId};

new_key_type! {
    pub struct RenderId;
}

pub struct RenderNode {
    pub widget_id: WidgetId,
    
    pub rect: Rect,
    
    pub canvas: Canvas,
}
