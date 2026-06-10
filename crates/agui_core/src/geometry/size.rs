use std::{
    fmt,
    num::FpCategory,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Rem, RemAssign, Sub, SubAssign},
};

use typed_floats::{NonNaN, as_const};

use crate::geometry::{Axis, Offset, Rect};

/// Holds width and height values.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Size {
    pub width: NonNaN<f32>,
    pub height: NonNaN<f32>,
}

impl fmt::Debug for Size {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Size")
            .field("width", &self.width.get())
            .field("height", &self.height.get())
            .finish()
    }
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

    /// # Panics
    ///
    /// Panics if `width` or `height` is NaN.
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

    /// # Safety
    ///
    /// The caller must ensure that neither width nor height is NaN.
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
        assert!(
            !(self.width.classify() == FpCategory::Zero
                && rhs.width.classify() == FpCategory::Zero),
            "cannot divide a zero width by a zero width"
        );

        assert!(
            !(self.width.is_infinite() && rhs.width.is_infinite()),
            "cannot divide an infinite width by an infinite width"
        );

        assert!(
            !(self.height.classify() == FpCategory::Zero
                && rhs.height.classify() == FpCategory::Zero),
            "cannot divide a zero height by a zero height"
        );

        assert!(
            !(self.height.is_infinite() && rhs.height.is_infinite()),
            "cannot divide an infinite height by an infinite height"
        );

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
        assert!(
            rhs.width.classify() != FpCategory::Zero,
            "cannot take the remainder of a width by zero"
        );

        assert!(
            !self.width.is_infinite(),
            "cannot take the remainder of an infinite width"
        );

        assert!(
            rhs.height.classify() != FpCategory::Zero,
            "cannot take the remainder of a height by zero"
        );

        assert!(
            !self.height.is_infinite(),
            "cannot take the remainder of an infinite height"
        );

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

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use super::*;

    #[test]
    fn add_finite_sizes() {
        let a = Size::new(10.0, 20.0);
        let b = Size::new(5.0, 3.0);
        let c = a + b;
        assert_eq!(c.width.get(), 15.0);
        assert_eq!(c.height.get(), 23.0);
    }

    #[test]
    #[should_panic(expected = "cannot add two opposite infinite widths")]
    fn add_opposite_infinite_widths_panics() {
        let a = Size::new(f32::INFINITY, 0.0);
        let b = Size::new(f32::NEG_INFINITY, 0.0);
        let _ = a + b;
    }

    #[test]
    #[should_panic(expected = "cannot add two opposite infinite heights")]
    fn add_opposite_infinite_heights_panics() {
        let a = Size::new(0.0, f32::INFINITY);
        let b = Size::new(0.0, f32::NEG_INFINITY);
        let _ = a + b;
    }

    #[test]
    fn sub_finite_sizes() {
        let a = Size::new(10.0, 20.0);
        let b = Size::new(3.0, 5.0);
        let c = a - b;
        assert_eq!(c.width.get(), 7.0);
        assert_eq!(c.height.get(), 15.0);
    }

    #[test]
    #[should_panic(expected = "cannot subtract two infinite widths of the same sign")]
    fn sub_same_sign_infinite_widths_panics() {
        let a = Size::new(f32::INFINITY, 0.0);
        let b = Size::new(f32::INFINITY, 0.0);
        let _ = a - b;
    }

    #[test]
    #[should_panic(expected = "cannot subtract two infinite heights of the same sign")]
    fn sub_same_sign_infinite_heights_panics() {
        let a = Size::new(0.0, f32::NEG_INFINITY);
        let b = Size::new(0.0, f32::NEG_INFINITY);
        let _ = a - b;
    }

    #[test]
    fn mul_finite_sizes() {
        let a = Size::new(3.0, 4.0);
        let b = Size::new(2.0, 5.0);
        let c = a * b;
        assert_eq!(c.width.get(), 6.0);
        assert_eq!(c.height.get(), 20.0);
    }

    #[test]
    #[should_panic(expected = "cannot multiply zero and zero width")]
    fn mul_zero_times_infinite_width_panics() {
        let a = Size::new(0.0, 1.0);
        let b = Size::new(f32::INFINITY, 1.0);
        let _ = a * b;
    }

    #[test]
    #[should_panic(expected = "cannot multiply zero and zero height")]
    fn mul_zero_times_infinite_height_panics() {
        let a = Size::new(1.0, 0.0);
        let b = Size::new(1.0, f32::INFINITY);
        let _ = a * b;
    }

    #[test]
    fn mul_by_scalar() {
        let a = Size::new(10.0, 20.0);
        let b = a * 3.0;
        assert_eq!(b.width.get(), 30.0);
        assert_eq!(b.height.get(), 60.0);
    }

    #[test]
    fn div_finite_sizes() {
        let a = Size::new(10.0, 20.0);
        let b = Size::new(2.0, 5.0);
        let c = a / b;
        assert_eq!(c.width.get(), 5.0);
        assert_eq!(c.height.get(), 4.0);
    }

    #[test]
    #[should_panic(expected = "cannot divide a zero width by a zero width")]
    fn div_zero_by_zero_width_panics() {
        let a = Size::new(0.0, 1.0);
        let b = Size::new(0.0, 1.0);
        let _ = a / b;
    }

    #[test]
    #[should_panic(expected = "cannot divide an infinite width by an infinite width")]
    fn div_infinite_by_infinite_width_panics() {
        let a = Size::new(f32::INFINITY, 1.0);
        let b = Size::new(f32::INFINITY, 1.0);
        let _ = a / b;
    }

    #[test]
    #[should_panic(expected = "cannot divide a zero height by a zero height")]
    fn div_zero_by_zero_height_panics() {
        let a = Size::new(1.0, 0.0);
        let b = Size::new(1.0, 0.0);
        let _ = a / b;
    }

    #[test]
    #[should_panic(expected = "cannot divide an infinite height by an infinite height")]
    fn div_infinite_by_infinite_height_panics() {
        let a = Size::new(1.0, f32::INFINITY);
        let b = Size::new(1.0, f32::INFINITY);
        let _ = a / b;
    }

    #[test]
    fn rem_finite_sizes() {
        let a = Size::new(10.0, 7.0);
        let b = Size::new(3.0, 4.0);
        let c = a % b;
        assert_eq!(c.width.get(), 1.0);
        assert_eq!(c.height.get(), 3.0);
    }

    #[test]
    #[should_panic(expected = "cannot take the remainder of a width by zero")]
    fn rem_by_zero_width_panics() {
        let a = Size::new(10.0, 1.0);
        let b = Size::new(0.0, 1.0);
        let _ = a % b;
    }

    #[test]
    #[should_panic(expected = "cannot take the remainder of an infinite width")]
    fn rem_of_infinite_width_panics() {
        let a = Size::new(f32::INFINITY, 1.0);
        let b = Size::new(3.0, 1.0);
        let _ = a % b;
    }

    #[test]
    #[should_panic(expected = "cannot take the remainder of a height by zero")]
    fn rem_by_zero_height_panics() {
        let a = Size::new(1.0, 10.0);
        let b = Size::new(1.0, 0.0);
        let _ = a % b;
    }

    #[test]
    fn rem_by_infinite_size_is_identity() {
        let a = Size::new(10.0, 7.0);
        let b = Size::new(f32::INFINITY, f32::INFINITY);
        let c = a % b;
        assert_eq!(c.width.get(), 10.0);
        assert_eq!(c.height.get(), 7.0);
    }

    #[test]
    fn neg_flips_sign() {
        let a = Size::new(10.0, -5.0);
        let b = -a;
        assert_eq!(b.width.get(), -10.0);
        assert_eq!(b.height.get(), 5.0);
    }

    #[test]
    fn is_zero_and_predicates() {
        assert!(Size::ZERO.is_zero());
        assert!(!Size::new(1.0, 1.0).is_zero());

        assert!(Size::new(1.0, 1.0).is_positive());
        assert!(!Size::new(-1.0, 1.0).is_positive());

        assert!(Size::new(-1.0, -1.0).is_negative());
        assert!(!Size::new(1.0, -1.0).is_negative());

        assert!(Size::new(f32::INFINITY, f32::INFINITY).is_infinite());
        assert!(!Size::new(1.0, f32::INFINITY).is_infinite());

        assert!(Size::new(1.0, 1.0).is_finite());
        assert!(!Size::new(f32::INFINITY, 1.0).is_finite());
    }
}
