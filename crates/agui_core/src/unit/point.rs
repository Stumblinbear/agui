use super::{Rect, POS_MARGIN_OF_ERROR};

/// Holds x and y values.
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        ((self.x - other.x).abs() < POS_MARGIN_OF_ERROR)
            && ((self.y - other.y).abs() < POS_MARGIN_OF_ERROR)
    }
}

impl From<Rect> for Point {
    fn from(rect: Rect) -> Self {
        Self {
            x: rect.x,
            y: rect.y,
        }
    }
}
