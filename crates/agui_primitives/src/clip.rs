use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Rect, Shape},
    widget::{IntoWidget, Widget, WidgetPaint},
};
use agui_macros::PaintWidget;

#[derive(PaintWidget, Debug, Default)]
pub struct Clip {
    pub rect: Option<Rect>,

    pub shape: Shape,
    pub anti_alias: bool,

    pub child: Option<Widget>,
}

impl Clip {
    pub const fn new() -> Self {
        Self {
            rect: None,

            shape: Shape::Rect,
            anti_alias: false,

            child: None,
        }
    }

    pub const fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);

        self
    }

    pub fn with_shape(mut self, shape: Shape) -> Self {
        self.shape = shape;

        self
    }

    pub const fn with_anti_alias(mut self, anti_alias: bool) -> Self {
        self.anti_alias = anti_alias;

        self
    }

    pub fn with_child<T: IntoWidget>(mut self, child: impl Into<Option<T>>) -> Self {
        self.child = child.into().map(IntoWidget::into_widget);

        self
    }
}

impl WidgetPaint for Clip {
    fn get_child(&self) -> Option<Widget> {
        self.child.clone()
    }

    fn paint(&self, mut canvas: CanvasPainter) {
        let brush = canvas.add_paint(Paint {
            anti_alias: self.anti_alias,
            ..Paint::default()
        });

        match self.rect {
            Some(rect) => canvas.start_layer_at(rect, &brush, self.shape.clone()),
            None => canvas.start_layer(&brush, self.shape.clone()),
        };
    }
}
