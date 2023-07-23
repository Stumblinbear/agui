use agui_core::{
    render::{CanvasPainter, Paint},
    unit::Color,
    widget::{WidgetPaint, WidgetRef},
};
use agui_macros::PaintWidget;

#[derive(PaintWidget, Debug, Default)]
pub struct ColoredBox {
    pub color: Color,

    #[child]
    pub child: WidgetRef,
}

impl WidgetPaint for ColoredBox {
    fn paint(&self, mut canvas: CanvasPainter) {
        canvas.draw_rect(&Paint {
            color: self.color,

            ..Paint::default()
        });
    }
}
