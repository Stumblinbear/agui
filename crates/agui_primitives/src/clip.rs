use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Rect, Shape},
    widget::{Widget, WidgetPaint},
};
use agui_macros::PaintWidget;

#[derive(PaintWidget, Debug, Default)]
pub struct Clip {
    pub rect: Option<Rect>,

    pub shape: Shape,
    pub anti_alias: bool,

    #[child]
    pub child: Option<Widget>,
}

impl WidgetPaint for Clip {
    fn paint(&self, canvas: CanvasPainter) {
        let paint = Paint {
            anti_alias: self.anti_alias,
            ..Paint::default()
        };

        match self.rect {
            Some(rect) => canvas.start_layer_at(rect, &paint, self.shape.clone()),
            None => canvas.start_layer(&paint, self.shape.clone()),
        };
    }
}
