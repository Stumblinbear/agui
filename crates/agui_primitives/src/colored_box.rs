use agui_core::{
    render::{CanvasPainter, Paint},
    unit::Color,
    widget::{Widget, WidgetPaint},
};
use agui_macros::PaintWidget;

#[derive(PaintWidget, Debug, Default)]
pub struct ColoredBox {
    pub color: Color,

    #[child]
    pub child: Option<Widget>,
}

impl WidgetPaint for ColoredBox {
    fn paint(&self, mut canvas: CanvasPainter) {
        canvas.draw_rect(&Paint {
            color: self.color,

            ..Paint::default()
        });
    }
}
