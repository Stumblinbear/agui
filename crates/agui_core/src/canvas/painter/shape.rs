use crate::{
    canvas::{clipping::Clip, paint::Paint, Canvas},
    unit::{Color, Shape},
};

use super::CanvasPainter;

pub struct RectPainter {
    pub color: Color,

    pub clip: Option<Clip>,
}

impl CanvasPainter for RectPainter {
    fn draw(&self, canvas: &mut Canvas) {
        let brush = canvas.new_brush(Paint { color: self.color });

        canvas.draw_rect(brush);

        if let Some(clip) = &self.clip {
            canvas.start_clipping(*clip, Shape::Rect);
        }
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
