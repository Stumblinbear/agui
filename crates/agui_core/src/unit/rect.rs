use super::{Size, POS_MARGIN_OF_ERROR};

/// Holds exact position and size values.
#[derive(Debug, Default, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PartialEq for Rect {
    fn eq(&self, other: &Self) -> bool {
        ((self.x - other.x).abs() < POS_MARGIN_OF_ERROR)
            && ((self.y - other.y).abs() < POS_MARGIN_OF_ERROR)
            && ((self.width - other.width).abs() < POS_MARGIN_OF_ERROR)
            && ((self.height - other.height).abs() < POS_MARGIN_OF_ERROR)
    }
}

impl Rect {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.x && point.0 <= self.x + self.width)
            && (point.1 >= self.y && point.1 <= self.y + self.height)
    }

    pub const fn to_slice(self) -> [f32; 4] {
        [self.x, self.y, self.width, self.height]
    }

    pub const fn normalize(self) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: self.width,
            height: self.height,
        }
    }
}

impl From<Size> for Rect {
    fn from(size: Size) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::unit::POS_MARGIN_OF_ERROR;

    use super::Rect;

    #[test]
    fn equality_test() {
        let rect1 = Rect {
            x: 0.1,
            y: 0.2,
            width: 0.2,
            height: 0.1,
        };

        let rect2 = Rect {
            x: 0.1 + POS_MARGIN_OF_ERROR,
            y: 0.2,
            width: 0.2,
            height: 0.1,
        };

        let rect3 = Rect {
            x: 1.0,
            y: 0.2,
            width: 0.2,
            height: 0.1,
        };

        assert_eq!(rect1, rect1, "rects should be equal");
        assert_eq!(rect1, rect2, "rects should be equal");
        assert_ne!(rect1, rect3, "rects should not be equal");
    }
}
