use agui_core::{
    render::{CanvasPainter, Paint},
    unit::Color,
    widget::{IntoWidget, Widget, WidgetPaint},
};
use agui_macros::PaintWidget;

#[derive(PaintWidget, Debug, Default)]
pub struct ColoredBox {
    pub color: Color,

    pub child: Option<Widget>,
}

impl ColoredBox {
    pub const fn new(color: Color) -> Self {
        Self { color, child: None }
    }

    pub fn with_child<T: IntoWidget>(mut self, child: impl Into<Option<T>>) -> Self {
        self.child = child.into().map(IntoWidget::into_widget);

        self
    }
}

impl WidgetPaint for ColoredBox {
    fn get_child(&self) -> Option<Widget> {
        self.child.clone()
    }

    fn paint(&self, mut canvas: CanvasPainter) {
        let brush = canvas.add_paint(Paint {
            color: self.color,

            ..Paint::default()
        });

        canvas.draw_rect(&brush);
    }
}
