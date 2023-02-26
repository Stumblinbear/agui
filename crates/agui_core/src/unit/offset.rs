use std::ops::{
    Add, AddAssign, BitAnd, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use super::{Rect, Size};

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Offset {
    pub x: f32,
    pub y: f32,
}

impl Offset {
    pub const ZERO: Self = Self::new(0.0, 0.0);
    pub const INFINITE: Self = Self::new(f32::INFINITY, f32::INFINITY);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_squared(&self) -> f32 {
        self.x.powf(2.0) + self.y.powf(2.0)
    }

    pub fn distance(&self) -> f32 {
        self.distance_squared().sqrt()
    }

    pub fn direction(&self) -> f32 {
        (self.y).atan2(self.x)
    }

    pub fn scale(&self, scale_x: f32, scale_y: f32) -> Self {
        Self::new(self.x * scale_x, self.y * scale_y)
    }

    pub fn translate(&self, x: f32, y: f32) -> Self {
        Self::new(self.x + x, self.y + y)
    }
}

impl Neg for Offset {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Add for Offset {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for Offset {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Offset {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for Offset {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul for Offset {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl MulAssign for Offset {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl Div for Offset {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl DivAssign for Offset {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl Rem for Offset {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::new(self.x % rhs.x, self.y % rhs.y)
    }
}

impl RemAssign for Offset {
    fn rem_assign(&mut self, rhs: Self) {
        self.x %= rhs.x;
        self.y %= rhs.y;
    }
}

impl BitAnd<Size> for Offset {
    type Output = Rect;

    fn bitand(self, rhs: Size) -> Self::Output {
        Rect::new(self.x, self.y, rhs.width, rhs.height)
    }
}

impl From<Rect> for Offset {
    fn from(rect: Rect) -> Self {
        Self::new(rect.x, rect.y)
    }
}

impl From<(f32, f32)> for Offset {
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}
