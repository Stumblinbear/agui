use crate::{canvas::Canvas, unit::Color};

use super::Painter;

pub struct RectPainter {
    pub color: Color,
}

impl Painter for RectPainter {
    fn paint(&self, canvas: &mut Canvas) {
        canvas.draw_rect(self.color);
    }
}

pub struct RoundedRectPainter {
    pub color: Color,

    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl Painter for RoundedRectPainter {
    fn paint(&self, canvas: &mut Canvas) {
        canvas.draw_rounded_rect(
            self.color,
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        );
    }
}
