use super::Size;

/// Holds exact position and size values.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub const fn new(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self {
            left,
            top,
            width,
            height,
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
            left: 0.0,
            top: 0.0,
            width: size.width,
            height: size.height,
        }
    }
}
