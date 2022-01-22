use crate::{
    canvas::{paint::Paint, Canvas},
    unit::Color,
};

use super::CanvasPainter;

pub struct RectPainter {
    pub color: Color,
}

impl CanvasPainter for RectPainter {
    fn draw(&self, canvas: &mut Canvas) {
        let brush = canvas.new_brush(Paint { color: self.color });

        canvas.draw_rect(brush);
    }
}

pub struct RoundedRectPainter {
    pub color: Color,

    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CanvasPainter for RoundedRectPainter {
    fn draw(&self, canvas: &mut Canvas) {
        let brush = canvas.new_brush(Paint { color: self.color });

        canvas.draw_rounded_rect(
            brush,
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        );
    }
}
