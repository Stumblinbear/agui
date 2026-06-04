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
    /// # Panics
    ///
    /// Panics if `left` or `top` is infinite or NaN, or if `width` or `height` is NaN.
    pub fn new<P, S>(left: P, top: P, width: S, height: S) -> Self
    where
        NonNaNFinite<f32>: TryFrom<P>,
        <NonNaNFinite<f32> as TryFrom<P>>::Error: std::fmt::Debug,
        NonNaN<f32>: TryFrom<S>,
        <NonNaN<f32> as TryFrom<S>>::Error: std::fmt::Debug,
    {
        Self {
            left: NonNaNFinite::try_from(left).expect("left must be a finite number"),
            top: NonNaNFinite::try_from(top).expect("top must be a finite number"),
            width: NonNaN::try_from(width).expect("width must not be NaN"),
            height: NonNaN::try_from(height).expect("height must not be NaN"),
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
