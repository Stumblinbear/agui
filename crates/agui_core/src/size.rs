use std::{
    num::FpCategory,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
};

use typed_floats::{as_const, NonNaN};

use crate::{axis::Axis, offset::Offset, rect::Rect};

/// Holds width and height values.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Size {
    pub width: NonNaN<f32>,
    pub height: NonNaN<f32>,
}

impl Default for Size {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Size {
    pub const ZERO: Self = Self {
        width: as_const!(NonNaN, f32, 0.0),
        height: as_const!(NonNaN, f32, 0.0),
    };

    #[inline]
    pub fn new<T>(width: T, height: T) -> Self
    where
        NonNaN<f32>: TryFrom<T>,
        <NonNaN<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self {
            width: NonNaN::try_from(width).expect("width must not be NaN"),
            height: NonNaN::try_from(height).expect("height must not be NaN"),
        }
    }

    pub unsafe fn new_unchecked(width: f32, height: f32) -> Self {
        unsafe {
            Self {
                width: NonNaN::<f32>::new_unchecked(width),
                height: NonNaN::<f32>::new_unchecked(height),
            }
        }
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

    pub fn extent(&self, axis: Axis) -> NonNaN<f32> {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }

    pub fn contains(&self, offset: Offset) -> bool {
        offset.x >= 0.0 && offset.y >= 0.0 && offset.x <= self.width && offset.y <= self.height
    }
}

impl Neg for Size {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            width: -self.width,
            height: -self.height,
        }
    }
}

impl Add for Size {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if self.width.is_infinite()
            && rhs.width.is_infinite()
            && self.width.is_sign_positive() != rhs.width.is_sign_positive()
        {
            panic!("cannot add two opposite infinite widths");
        }

        if self.height.is_infinite()
            && rhs.height.is_infinite()
            && self.height.is_sign_positive() != rhs.height.is_sign_positive()
        {
            panic!("cannot add two opposite infinite heights");
        }

        Self::new(self.width + rhs.width, self.height + rhs.height)
    }
}

impl AddAssign for Size {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub for Size {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.width.is_infinite()
            && rhs.width.is_infinite()
            && self.width.is_sign_positive() == rhs.width.is_sign_positive()
        {
            panic!("cannot subtract two infinite widths of the same sign");
        }

        if self.height.is_infinite()
            && rhs.height.is_infinite()
            && self.height.is_sign_positive() == rhs.height.is_sign_positive()
        {
            panic!("cannot subtract two infinite heights of the same sign");
        }

        Self::new(self.width - rhs.width, self.height - rhs.height)
    }
}

impl SubAssign for Size {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Mul for Size {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        if (self.width.classify() == FpCategory::Zero || rhs.width.classify() == FpCategory::Zero)
            && (self.width.is_infinite() || rhs.width.is_infinite())
        {
            panic!("cannot multiply zero and zero width");
        }

        if (self.height.classify() == FpCategory::Zero || rhs.height.classify() == FpCategory::Zero)
            && (self.height.is_infinite() || rhs.height.is_infinite())
        {
            panic!("cannot multiply zero and zero height");
        }

        Self::new(self.width * rhs.width, self.height * rhs.height)
    }
}

impl MulAssign for Size {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl Mul<f32> for Size {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        self * Self::new(rhs, rhs)
    }
}

impl MulAssign<f32> for Size {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * Self::new(rhs, rhs);
    }
}

impl Div for Size {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        if self.width.classify() == FpCategory::Zero && rhs.width.classify() == FpCategory::Zero {
            panic!("cannot divide a zero width by a zero width");
        }

        if self.width.is_infinite() && rhs.width.is_infinite() {
            panic!("cannot divide an infinite width by an infinite width");
        }

        if self.height.classify() == FpCategory::Zero && rhs.height.classify() == FpCategory::Zero {
            panic!("cannot divide a zero height by a zero height");
        }

        if self.height.is_infinite() && rhs.height.is_infinite() {
            panic!("cannot divide an infinite height by an infinite height");
        }

        Self::new(self.width / rhs.width, self.height / rhs.height)
    }
}

impl DivAssign for Size {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Div<f32> for Size {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        self / Self::new(rhs, rhs)
    }
}

impl DivAssign<f32> for Size {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / Self::new(rhs, rhs);
    }
}

impl Rem for Size {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        if self.width.classify() == FpCategory::Zero && rhs.width.classify() == FpCategory::Zero {
            panic!("cannot divide a zero width by a zero width");
        }

        if self.width.is_infinite() && rhs.width.is_infinite() {
            panic!("cannot divide an infinite width by an infinite width");
        }

        if self.height.classify() == FpCategory::Zero && rhs.height.classify() == FpCategory::Zero {
            panic!("cannot divide a zero height by a zero height");
        }

        if self.height.is_infinite() && rhs.height.is_infinite() {
            panic!("cannot divide an infinite height by an infinite height");
        }

        Self::new(self.width % rhs.width, self.height % rhs.height)
    }
}

impl RemAssign for Size {
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl Rem<f32> for Size {
    type Output = Self;

    fn rem(self, rhs: f32) -> Self::Output {
        self % Self::new(rhs, rhs)
    }
}

impl RemAssign<f32> for Size {
    fn rem_assign(&mut self, rhs: f32) {
        *self = *self % Self::new(rhs, rhs);
    }
}

impl From<Rect> for Size {
    fn from(rect: Rect) -> Self {
        Self::new(rect.width, rect.height)
    }
}
