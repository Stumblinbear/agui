use typed_floats::{NonNaN, NonNaNFinite, as_const};

use crate::size::Size;

/// Holds exact position and size values.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rect {
    pub left: NonNaNFinite<f32>,
    pub top: NonNaNFinite<f32>,
    pub width: NonNaN<f32>,
    pub height: NonNaN<f32>,
}

impl Rect {
    pub fn new(
        left: impl Into<NonNaNFinite<f32>>,
        top: impl Into<NonNaNFinite<f32>>,
        width: impl Into<NonNaN<f32>>,
        height: impl Into<NonNaN<f32>>,
    ) -> Self {
        Self {
            left: left.into(),
            top: top.into(),
            width: width.into(),
            height: height.into(),
        }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.left && point.0 <= self.left + self.width)
            && (point.1 >= self.top && point.1 <= self.top + self.height)
    }
}

impl From<Size> for Rect {
    fn from(size: Size) -> Self {
        Self {
            left: as_const!(NonNaNFinite, f32, 0.0),
            top: as_const!(NonNaNFinite, f32, 0.0),
            width: size.width,
            height: size.height,
        }
    }
}
