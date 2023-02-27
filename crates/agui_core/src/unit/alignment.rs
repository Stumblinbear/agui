use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use super::{Offset, Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Alignment {
    pub x: f32,
    pub y: f32,
}

impl Default for Alignment {
    fn default() -> Self {
        Self::CENTER
    }
}

impl Alignment {
    /// The top left corner.
    pub const TOP_LEFT: Alignment = Alignment::new(-1.0, -1.0);

    /// The center point along the top edge.
    pub const TOP_CENTER: Alignment = Alignment::new(0.0, -1.0);

    /// The top right corner.
    pub const TOP_RIGHT: Alignment = Alignment::new(1.0, -1.0);

    /// The center point along the left edge.
    pub const CENTER_LEFT: Alignment = Alignment::new(-1.0, 0.0);

    /// The center point, both horizontally and vertically.
    pub const CENTER: Alignment = Alignment::new(0.0, 0.0);

    /// The center point along the right edge.
    pub const CENTER_RIGHT: Alignment = Alignment::new(1.0, 0.0);

    /// The bottom left corner.
    pub const BOTTOM_LEFT: Alignment = Alignment::new(-1.0, 1.0);

    /// The center point along the bottom edge.
    pub const BOTTOM_CENTER: Alignment = Alignment::new(0.0, 1.0);

    /// The bottom right corner.
    pub const BOTTOM_RIGHT: Alignment = Alignment::new(1.0, 1.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the offset that is this fraction in the direction of the given offset.
    pub fn along_offset(&self, offset: Offset) -> Offset {
        let center = offset / 2.0;

        Offset::new(center.x + self.x * center.x, center.y + self.y * center.y)
    }

    /// Returns the offset that is this fraction within the given size.
    pub fn along_size(&self, size: Size) -> Offset {
        let half_size = size / 2.0;

        Offset::new(
            half_size.width + self.x * half_size.width,
            half_size.height + self.y * half_size.height,
        )
    }

    /// Returns the point that is this fraction within the given rect.
    pub fn within_rect(&self, rect: Rect) -> Offset {
        let half_size = Size::from(rect) / 2.0;

        Offset::new(
            rect.left + half_size.width + self.x * half_size.width,
            rect.top + half_size.height + self.y * half_size.height,
        )
    }

    /// Returns a rect of the given size, aligned within given rect as specified
    /// by this alignment.
    ///
    /// For example, a 100×100 size inscribed on a 200×200 rect using
    /// [Alignment.topLeft] would be the 100×100 rect at the top left of
    /// the 200×200 rect.
    pub fn inscribe(&self, size: Size, rect: Rect) -> Rect {
        let half_size_delta = (Size::from(rect) - size) / 2.0;

        Rect {
            left: rect.left + half_size_delta.width + self.x * half_size_delta.width,
            top: rect.top + half_size_delta.height + self.y * half_size_delta.height,
            width: size.width,
            height: size.height,
        }
    }
}

impl Neg for Alignment {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Add for Alignment {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Alignment {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Alignment {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Alignment {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul for Alignment {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl MulAssign for Alignment {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl Div for Alignment {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl DivAssign for Alignment {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl Rem for Alignment {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::new(self.x % rhs.x, self.y % rhs.y)
    }
}

impl RemAssign for Alignment {
    fn rem_assign(&mut self, rhs: Self) {
        self.x %= rhs.x;
        self.y %= rhs.y;
    }
}
