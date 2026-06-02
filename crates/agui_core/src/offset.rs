use std::ops::{
    Add, AddAssign, BitAnd, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign,
};

use glam::{Vec2, Vec3};

use typed_floats::{Atan2, NonNaN, NonNaNFinite, Positive, Powf, as_const};

use crate::{edge_insets::EdgeInsets, rect::Rect, size::Size};

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Offset {
    pub x: NonNaNFinite<f32>,
    pub y: NonNaNFinite<f32>,
}

impl Offset {
    pub const ZERO: Self = Self {
        x: as_const!(NonNaNFinite, f32, 0.0),
        y: as_const!(NonNaNFinite, f32, 0.0),
    };

    /// # Panics
    ///
    /// Panics if `x` or `y` is infinite or NaN.
    pub fn new<T>(x: T, y: T) -> Self
    where
        NonNaNFinite<f32>: TryFrom<T>,
        <NonNaNFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self {
            x: NonNaNFinite::try_from(x).expect("x must be a finite number"),
            y: NonNaNFinite::try_from(y).expect("y must be a finite number"),
        }
    }

    pub fn distance_squared(self) -> Positive<f32> {
        // SAFETY: x^2 and y^2 are positive numbers, so the result is also positive.
        unsafe {
            Positive::<f32>::new_unchecked(
                self.x.powf(as_const!(NonNaNFinite, f32, 2.0))
                    + self.y.powf(as_const!(NonNaNFinite, f32, 2.0)),
            )
        }
    }

    pub fn distance(self) -> Positive<f32> {
        self.distance_squared().sqrt()
    }

    pub fn direction(self) -> NonNaNFinite<f32> {
        self.y.atan2(self.x)
    }

    /// # Panics
    ///
    /// Panics if either scalar is infinite or NaN.
    pub fn scale<T>(self, scale_x: T, scale_y: T) -> Self
    where
        NonNaNFinite<f32>: TryFrom<T>,
        <NonNaNFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self::new::<NonNaN<f32>>(
            self.x * NonNaNFinite::try_from(scale_x).expect("x scalar must be a finite number"),
            self.y * NonNaNFinite::try_from(scale_y).expect("y scalar must be a finite number"),
        )
    }

    /// # Panics
    ///
    /// Panics if `x` or `y` is infinite or NaN.
    pub fn translate<T>(self, x: T, y: T) -> Self
    where
        NonNaNFinite<f32>: TryFrom<T>,
        <NonNaNFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self::new::<NonNaN<f32>>(
            self.x + NonNaNFinite::try_from(x).expect("x must be a finite number"),
            self.y + NonNaNFinite::try_from(y).expect("y must be a finite number"),
        )
    }
}

impl Neg for Offset {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            x: -self.x,
            y: -self.y,
        }
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
        *self = *self + rhs;
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
        *self = *self - rhs;
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
        *self = *self * rhs;
    }
}

impl Mul<f32> for Offset {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        let rhs = NonNaNFinite::try_from(rhs).expect("rhs must be a finite number");

        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<f32> for Offset {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
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
        *self = *self / rhs;
    }
}

impl Div<f32> for Offset {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        let rhs = NonNaNFinite::try_from(rhs).expect("rhs must be a finite number");

        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<f32> for Offset {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
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
        *self = *self % rhs;
    }
}

impl Rem<f32> for Offset {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self::Output {
        let rhs = NonNaNFinite::try_from(rhs).expect("rhs must be a finite number");

        Self::new(self.x % rhs, self.y % rhs)
    }
}

impl RemAssign<f32> for Offset {
    fn rem_assign(&mut self, rhs: f32) {
        *self = *self % rhs;
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
        Self::new(rect.left, rect.top)
    }
}

impl From<EdgeInsets> for Offset {
    fn from(insets: EdgeInsets) -> Self {
        Self::new(insets.left, insets.top)
    }
}

impl From<(f32, f32)> for Offset {
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}

impl From<Vec2> for Offset {
    fn from(value: Vec2) -> Self {
        Self::new(value.x, value.y)
    }
}

impl From<Offset> for Vec2 {
    fn from(val: Offset) -> Self {
        Vec2::new(val.x.get(), val.y.get())
    }
}

impl From<Offset> for Vec3 {
    fn from(val: Offset) -> Self {
        Vec3::new(val.x.get(), val.y.get(), 0.0)
    }
}
