use std::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use super::{Axis, Rect};

/// Holds width and height values.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn is_zero(&self) -> bool {
        self == &Size::ZERO
    }

    pub fn is_positive(&self) -> bool {
        self.width > 0.0 && self.height > 0.0
    }

    pub fn is_negative(&self) -> bool {
        self.width < 0.0 && self.height < 0.0
    }

    pub fn is_infinite(&self) -> bool {
        self.width.is_infinite() && self.height.is_infinite()
    }

    pub fn is_finite(&self) -> bool {
        self.width.is_finite() && self.height.is_finite()
    }

    pub fn get_extent(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.width, -self.height)
    }
}

impl Add for Size {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.width + rhs.width, self.height + rhs.height)
    }
}

impl AddAssign for Size {
    fn add_assign(&mut self, rhs: Self) {
        self.width += rhs.width;
        self.height += rhs.height;
    }
}

impl Sub for Size {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.width - rhs.width, self.height - rhs.height)
    }
}

impl SubAssign for Size {
    fn sub_assign(&mut self, rhs: Self) {
        self.width -= rhs.width;
        self.height -= rhs.height;
    }
}

impl Mul for Size {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.width * rhs.width, self.height * rhs.height)
    }
}

impl MulAssign for Size {
    fn mul_assign(&mut self, rhs: Self) {
        self.width *= rhs.width;
        self.height *= rhs.height;
    }
}

impl Mul<f32> for Size {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.width * rhs, self.height * rhs)
    }
}

impl MulAssign<f32> for Size {
    fn mul_assign(&mut self, rhs: f32) {
        self.width *= rhs;
        self.height *= rhs;
    }
}

impl Div for Size {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.width / rhs.width, self.height / rhs.height)
    }
}

impl DivAssign for Size {
    fn div_assign(&mut self, rhs: Self) {
        self.width /= rhs.width;
        self.height /= rhs.height;
    }
}

impl Div<f32> for Size {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.width / rhs, self.height / rhs)
    }
}

impl DivAssign<f32> for Size {
    fn div_assign(&mut self, rhs: f32) {
        self.width /= rhs;
        self.height /= rhs;
    }
}

impl Rem for Size {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::new(self.width % rhs.width, self.height % rhs.height)
    }
}

impl RemAssign for Size {
    fn rem_assign(&mut self, rhs: Self) {
        self.width %= rhs.width;
        self.height %= rhs.height;
    }
}

impl Rem<f32> for Size {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self::Output {
        Self::new(self.width % rhs, self.height % rhs)
    }
}

impl RemAssign<f32> for Size {
    fn rem_assign(&mut self, rhs: f32) {
        self.width %= rhs;
        self.height %= rhs;
    }
}

impl From<Rect> for Size {
    fn from(rect: Rect) -> Self {
        Self::new(rect.width, rect.height)
    }
}

impl From<(f32, f32)> for Size {
    fn from((width, height): (f32, f32)) -> Self {
        Self { width, height }
    }
}
