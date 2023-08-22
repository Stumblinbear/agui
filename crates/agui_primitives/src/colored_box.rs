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
        let brush = canvas.add_paint(Paint {
            color: self.color,

            ..Paint::default()
        });

        canvas.draw_rect(&brush);
    }
}
