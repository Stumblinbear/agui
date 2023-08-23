use agui_core::{
    render::{CanvasPainter, Paint},
    unit::Color,
    widget::{IntoChild, Widget, WidgetPaint},
};
use agui_macros::PaintWidget;

#[derive(PaintWidget, Debug, Default)]
pub struct ColoredBox {
    pub color: Color,

    #[child]
    pub child: Option<Widget>,
}

impl ColoredBox {
    pub fn new(color: Color) -> Self {
        Self { color, child: None }
    }

    pub fn with_child(mut self, child: impl IntoChild) -> Self {
        self.child = child.into_child();

        self
    }
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
