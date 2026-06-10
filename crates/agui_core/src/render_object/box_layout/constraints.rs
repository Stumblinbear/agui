use std::{
    fmt,
    ops::{Div, Mul, MulAssign},
};

use typed_floats::{Positive, StrictlyPositiveFinite, as_const};

use crate::geometry::{Axis, EdgeInsetsGeometry, Size};

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct BoxConstraints {
    min_width: Positive<f32>,
    max_width: Positive<f32>,
    min_height: Positive<f32>,
    max_height: Positive<f32>,
}

impl fmt::Debug for BoxConstraints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn axis(
            f: &mut fmt::Formatter<'_>,
            label: char,
            min: Positive<f32>,
            max: Positive<f32>,
        ) -> fmt::Result {
            if min == max {
                return write!(f, "{label}={:?}", min.get());
            }

            write!(f, "{:?}<={label}<=", min.get())?;

            if max.is_infinite() {
                f.write_str("inf")
            } else {
                write!(f, "{:?}", max.get())
            }
        }

        axis(f, 'w', self.min_width, self.max_width)?;
        f.write_str(", ")?;
        axis(f, 'h', self.min_height, self.max_height)
    }
}

impl Default for BoxConstraints {
    fn default() -> Self {
        Self {
            min_width: as_const!(Positive, f32, 0.0),
            max_width: as_const!(Positive, f32, f32::INFINITY),
            min_height: as_const!(Positive, f32, 0.0),
            max_height: as_const!(Positive, f32, f32::INFINITY),
        }
    }
}

impl BoxConstraints {
    /// # Panics
    ///
    /// Panics if any bound is negative or NaN, or if a minimum exceeds its maximum.
    pub fn new<T>(min_width: T, max_width: T, min_height: T, max_height: T) -> Self
    where
        Positive<f32>: TryFrom<T>,
        <Positive<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        let min_width =
            Positive::try_from(min_width).expect("minimum width must be a non-negative number");
        let max_width =
            Positive::try_from(max_width).expect("maximum width must be a non-negative number");
        let min_height =
            Positive::try_from(min_height).expect("minimum height must be a non-negative number");
        let max_height =
            Positive::try_from(max_height).expect("maximum height must be a non-negative number");

        assert!(
            min_width <= max_width,
            "minimum width must not be greater than the maximum width"
        );

        assert!(
            min_height <= max_height,
            "minimum height must not be greater than the maximum height"
        );

        Self {
            min_width: min_width.min(max_width),
            max_width: max_width.max(min_width),
            min_height: min_height.min(max_height),
            max_height: max_height.max(min_height),
        }
    }

    /// Creates [`Constraints`] that require the given constraints only on the given axis, with
    /// the other axis unconstrained.
    ///
    /// # Panics
    ///
    /// Panics if `min` or `max` is negative or NaN, or if `min` exceeds `max`.
    pub fn along_axis<T>(axis: Axis, min: T, max: T) -> Self
    where
        Positive<f32>: TryFrom<T>,
        <Positive<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        let min =
            Positive::try_from(min).expect("minimum constraint must be a non-negative number");
        let max =
            Positive::try_from(max).expect("maximum constraint must be a non-negative number");

        assert!(
            min <= max,
            "minimum constraint must not be greater than the maximum constraint"
        );

        let min = min.min(max);
        let max = max.max(min);

        match axis {
            Axis::Horizontal => Self {
                min_width: min,
                max_width: max,
                ..Default::default()
            },
            Axis::Vertical => Self {
                min_height: min,
                max_height: max,
                ..Default::default()
            },
        }
    }

    /// Creates [`Constraints`] that require the given size.
    ///
    /// # Panics
    ///
    /// Panics if `size` has a negative dimension.
    pub fn tight(size: Size) -> Self {
        let width =
            Positive::try_from(size.width).expect("tight width must be a non-negative number");
        let height =
            Positive::try_from(size.height).expect("tight height must be a non-negative number");

        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }

    /// Creates [`Constraints`] that require the given width and/or height, leaving an unset
    /// dimension unconstrained.
    pub fn tight_for(width: Option<Positive<f32>>, height: Option<Positive<f32>>) -> Self {
        let mut constraints = Self::default();

        if let Some(width) = width {
            constraints = constraints.tighten_width(width.get());
        }

        if let Some(height) = height {
            constraints = constraints.tighten_height(height.get());
        }

        constraints
    }

    /// Creates [`Constraints`] that require the given size on the given axis.
    pub fn tight_for_axis<T>(axis: Axis, size: T) -> Self
    where
        T: Copy,
        Positive<f32>: TryFrom<T>,
        <Positive<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self::along_axis(axis, size, size)
    }

    /// Creates [`Constraints`] that forbids sizes larger than the given size.
    ///
    /// # Panics
    ///
    /// Panics if `size` has a negative dimension.
    pub fn loose(size: Size) -> Self {
        let max_width =
            Positive::try_from(size.width).expect("loose width must be a non-negative number");
        let max_height =
            Positive::try_from(size.height).expect("loose height must be a non-negative number");

        Self {
            min_width: as_const!(Positive, f32, 0.0),
            max_width,
            min_height: as_const!(Positive, f32, 0.0),
            max_height,
        }
    }

    /// Creates [`Constraints`] that forbids sizes larger than the given size on the given axis.
    ///
    /// # Panics
    ///
    /// Panics if `size` is negative or NaN.
    pub fn loose_for_axis<T>(axis: Axis, size: T) -> Self
    where
        Positive<f32>: TryFrom<T>,
        <Positive<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        let max: Positive<f32> =
            Positive::try_from(size).expect("loose size must be a non-negative number");

        Self::along_axis::<Positive<f32>>(axis, as_const!(Positive, f32, 0.0), max)
    }

    pub fn min_width(self) -> Positive<f32> {
        self.min_width
    }

    pub fn max_width(self) -> Positive<f32> {
        self.max_width
    }

    pub fn min_height(self) -> Positive<f32> {
        self.min_height
    }

    pub fn max_height(self) -> Positive<f32> {
        self.max_height
    }

    pub fn min_axis(self, axis: Axis) -> Positive<f32> {
        match axis {
            Axis::Horizontal => self.min_width,
            Axis::Vertical => self.min_height,
        }
    }

    pub fn max_axis(self, axis: Axis) -> Positive<f32> {
        match axis {
            Axis::Horizontal => self.max_width,
            Axis::Vertical => self.max_height,
        }
    }

    pub const fn expand() -> Self {
        Self {
            min_width: as_const!(Positive, f32, f32::INFINITY),
            max_width: as_const!(Positive, f32, f32::INFINITY),
            min_height: as_const!(Positive, f32, f32::INFINITY),
            max_height: as_const!(Positive, f32, f32::INFINITY),
        }
    }

    /// Returns new [`Constraints`] that are smaller by the given edge dimensions.
    pub fn deflate<T>(self, insets: &T) -> Self
    where
        T: EdgeInsetsGeometry,
    {
        let horizontal_size = insets.horizontal();
        let vertical_size = insets.vertical();

        let deflated_min_width = as_const!(NonNaN, f32, 0.0).max(self.min_width - horizontal_size);
        let deflated_max_width = deflated_min_width.max(self.max_width - horizontal_size);

        let deflated_min_height = as_const!(NonNaN, f32, 0.0).max(self.min_height - vertical_size);
        let deflated_max_height = deflated_min_height.max(self.max_height - vertical_size);

        Self {
            min_width: deflated_min_width.abs(),
            max_width: deflated_max_width.abs(),
            min_height: deflated_min_height.abs(),
            max_height: deflated_max_height.abs(),
        }
    }

    /// Returns new [`Constraints`] that remove the minimum width and height requirements.
    pub fn loosen(self) -> Self {
        Self {
            min_width: as_const!(Positive, f32, 0.0),
            max_width: self.max_width,
            min_height: as_const!(Positive, f32, 0.0),
            max_height: self.max_height,
        }
    }

    /// Returns new [`Constraints`] that respect the given constraints while being as
    /// close as possible to the original constraints.
    pub fn enforce(self, other: impl Into<BoxConstraints>) -> Self {
        let other = other.into();

        Self {
            min_width: self.min_width.clamp(other.min_width, other.max_width),
            max_width: self.max_width.clamp(other.min_width, other.max_width),
            min_height: self.min_height.clamp(other.min_height, other.max_height),
            max_height: self.max_height.clamp(other.min_height, other.max_height),
        }
    }

    /// Returns new [`Constraints`] with a tight width as close to the given width as
    /// possible while still respecting the original constraints.
    ///
    /// # Panics
    ///
    /// Panics if `width` is negative or NaN.
    pub fn tighten_width(self, width: f32) -> Self {
        let width = Positive::try_from(width).expect("tight width must be a non-negative number");

        Self {
            min_width: width.clamp(self.min_width, self.max_width),
            max_width: width.clamp(self.min_width, self.max_width),
            ..self
        }
    }

    /// Returns new [`Constraints`] with a tight height as close to the given height as
    /// possible while still respecting the original constraints.
    ///
    /// # Panics
    ///
    /// Panics if `height` is negative or NaN.
    pub fn tighten_height(self, height: f32) -> Self {
        let height =
            Positive::try_from(height).expect("tight height must be a non-negative number");

        Self {
            min_height: height.clamp(self.min_height, self.max_height),
            max_height: height.clamp(self.min_height, self.max_height),
            ..self
        }
    }

    /// Returns new [`Constraints`] with a tight width and/or height as close to the given
    /// size as possible while still respecting the original constraints.
    pub fn tighten_axis(self, axis: Axis, extent: f32) -> Self {
        match axis {
            Axis::Horizontal => self.tighten_width(extent),
            Axis::Vertical => self.tighten_height(extent),
        }
    }

    /// Returns new [`Constraints`] with a tight width and/or height as close to the given
    /// size as possible while still respecting the original constraints.
    pub fn tighten(self, other: impl Into<Size>) -> Self {
        let other: Size = other.into();

        let width = other.width;
        let height = other.height;

        // Since we carry non-negative numbers and [`Size`] is non-NaN, clamp will nearly always return positive numbers.
        // The exception is on some architectures where -0.0 may be produced: we resolve this by calling abs().
        Self {
            min_width: width
                .clamp(self.min_width.into(), self.max_width.into())
                .abs(),
            max_width: width
                .clamp(self.min_width.into(), self.max_width.into())
                .abs(),

            min_height: height
                .clamp(self.min_height.into(), self.max_height.into())
                .abs(),
            max_height: height
                .clamp(self.min_height.into(), self.max_height.into())
                .abs(),
        }
    }

    /// Returns new [`Constraints`] with the width and height constraints flipped.
    pub fn flip(self) -> Self {
        Self {
            min_width: self.min_height,
            max_width: self.max_height,
            min_height: self.min_width,
            max_height: self.max_width,
        }
    }

    /// Returns new [`Constraints`] with the same constraints on the given axis but with
    /// the opposite axis unconstrained.
    pub fn only_along(self, axis: Axis) -> Self {
        match axis {
            Axis::Horizontal => self.only_width(),
            Axis::Vertical => self.only_height(),
        }
    }

    /// Returns new [`Constraints`] with the same width constraints but with unconstrained
    /// height.
    pub fn only_width(self) -> Self {
        Self::along_axis(Axis::Horizontal, self.min_width.get(), self.max_width.get())
    }

    /// Returns new [`Constraints`] with the same height constraints but with unconstrained
    /// width.
    pub fn only_height(self) -> Self {
        Self::along_axis(Axis::Vertical, self.min_height.get(), self.max_height.get())
    }

    /// Returns the width that both satisfies the constraints and is as close as
    /// possible to the given width.
    ///
    /// # Panics
    ///
    /// Panics if `width` is negative or NaN.
    pub fn constrain_width(self, width: f32) -> Positive<f32> {
        let width =
            Positive::try_from(width).expect("constrained width must be a non-negative number");

        width.clamp(self.min_width, self.max_width)
    }

    /// Returns the height that both satisfies the constraints and is as close as
    /// possible to the given height.
    ///
    /// # Panics
    ///
    /// Panics if `height` is negative or NaN.
    pub fn constrain_height(self, height: f32) -> Positive<f32> {
        let height =
            Positive::try_from(height).expect("constrained width must be a non-negative number");

        height.clamp(self.min_height, self.max_height)
    }

    /// Returns the height that both satisfies the constraints and is as close as
    /// possible to the given height.
    pub fn constrain_axis(self, axis: Axis, extent: f32) -> Positive<f32> {
        match axis {
            Axis::Horizontal => self.constrain_width(extent),
            Axis::Vertical => self.constrain_height(extent),
        }
    }

    /// Returns the [Size] that both satisfies the constraints and is as close as
    /// possible to the given size.
    pub fn constrain(self, size: impl Into<Size>) -> Size {
        let size: Size = size.into();

        Size::new(
            self.constrain_width(size.width.get()),
            self.constrain_height(size.height.get()),
        )
    }

    /// Returns a [Size] that attempts to meet the following conditions, in order:
    ///
    ///  * The size must satisfy these constraints.
    ///  * The aspect ratio of the returned size matches the aspect ratio of the
    ///    given size.
    ///  * The returned size as big as possible while still being equal to or
    ///    smaller than the given size.
    ///
    /// # Panics
    ///
    /// Panics if the size's aspect ratio is not finite and greater than zero, such as when its
    /// height is zero.
    pub fn constrain_preserve_aspect_ratio(self, size: impl Into<Size>) -> Size {
        let size: Size = size.into();

        let mut width = size.width;
        let mut height = size.height;

        let aspect_ratio = StrictlyPositiveFinite::try_from(size.width / size.height)
            .expect("aspect ratio must be a finite number greater than zero");

        if width > self.max_width {
            width = self.max_width.into();
            height = width / aspect_ratio;
        }

        if height > self.max_height {
            height = self.max_height.into();
            width = height * aspect_ratio;
        }

        if width < self.min_width {
            width = self.min_width.into();
            height = width / aspect_ratio;
        }

        if height < self.min_height {
            height = self.min_height.into();
            width = height * aspect_ratio;
        }

        Size::new(
            self.constrain_width(width.get()),
            self.constrain_height(height.get()),
        )
    }

    /// The smallest [Size] that satisfies the constraints.
    pub fn smallest(self) -> Size {
        Size::new(self.min_width, self.min_height)
    }

    /// The biggest [Size] that satisfies the constraints.
    pub fn biggest(self) -> Size {
        Size::new(self.max_width, self.max_height)
    }

    /// Whether there is exactly one width value that satisfies the constraints.
    pub fn has_tight_width(self) -> bool {
        self.min_width == self.max_width && self.has_bounded_width()
    }

    /// Whether there is exactly one height value that satisfies the constraints
    pub fn has_tight_height(self) -> bool {
        self.min_height == self.max_height && self.has_bounded_height()
    }

    /// Whether there is exactly one extent on the given axis that satisfies the constraints.
    pub fn has_tight_axis(self, axis: Axis) -> bool {
        match axis {
            Axis::Horizontal => self.has_tight_width(),
            Axis::Vertical => self.has_tight_height(),
        }
    }

    /// Whether there is exactly one [Size] that satisfies the constraints.
    pub fn is_tight(&self) -> bool {
        self.has_tight_width() && self.has_tight_height()
    }

    /// Whether there is an upper bound on the maximum width.
    ///
    /// See also:
    ///
    ///  * [`Constraints::has_bounded_height`], the equivalent for the vertical axis.
    ///  * [`Constraints::has_infinite_width`], which describes whether the minimum width
    ///    constraint is infinite.
    pub fn has_bounded_width(&self) -> bool {
        self.max_width < f32::INFINITY
    }

    /// Whether there is an upper bound on the maximum height.
    ///
    /// See also:
    ///
    ///  * [`Constraints::has_bounded_width`], the equivalent for the horizontal axis.
    ///  * [`Constraints::has_infinite_height`], which describes whether the minimum height
    ///    constraint is infinite.
    pub fn has_bounded_height(&self) -> bool {
        self.max_height < f32::INFINITY
    }

    /// Whether the width constraint is infinite.
    ///
    /// Such a constraint is used to indicate that a box should grow as large as
    /// some other constraint (in this case, horizontally). If constraints are
    /// infinite, then they must have other (non-infinite) constraints [enforce]d
    /// upon them, or must be [tighten]ed, before they can be used to derive a
    /// [Size] for a [RenderBox.size].
    ///
    /// See also:
    ///
    ///  * [`Constraints::has_infinite_height`], the equivalent for the vertical axis.
    ///  * [`Constraints::has_bounded_width`], which describes whether the maximum width
    ///    constraint is finite.
    pub fn has_infinite_width(&self) -> bool {
        self.min_width >= f32::INFINITY
    }

    /// Whether the height constraint is infinite.
    ///
    /// Such a constraint is used to indicate that a box should grow as large as
    /// some other constraint (in this case, vertically). If constraints are
    /// infinite, then they must have other (non-infinite) constraints [enforce]d
    /// upon them, or must be [tighten]ed, before they can be used to derive a
    /// [Size].
    ///
    /// See also:
    ///
    ///  * [`Constraints::has_infinite_width`], the equivalent for the horizontal axis.
    ///  * [`Constraints::has_bounded_height`], which describes whether the maximum height
    ///    constraint is finite.
    pub fn has_infinite_height(&self) -> bool {
        self.min_height >= f32::INFINITY
    }

    /// Whether the given [Size] satisfies the [Constraints].
    pub fn is_satisfied_by(&self, size: impl Into<Size>) -> bool {
        let size = size.into();

        size.width >= self.min_width
            && size.width <= self.max_width
            && size.height >= self.min_height
            && size.height <= self.max_height
    }

    // pub fn lerp(a: &Constraints, b: &Constraints, t: f32) -> Self {
    //     let t = Positive::try_from(t).expect("t must be a non-negative number");

    //     assert!(
    //         (a.min_width.is_finite() && b.min_width.is_finite())
    //             || (a.min_width.is_infinite() && b.min_width.is_infinite()),
    //         "Cannot interpolate between finite constraints and unbounded constraints"
    //     );
    //     assert!(
    //         (a.max_width.is_finite() && b.max_width.is_finite())
    //             || (a.max_width.is_infinite() && b.max_width.is_infinite()),
    //         "Cannot interpolate between finite constraints and unbounded constraints"
    //     );

    //     assert!(
    //         (a.min_height.is_finite() && b.min_height.is_finite())
    //             || (a.min_height.is_infinite() && b.min_height.is_infinite()),
    //         "Cannot interpolate between finite constraints and unbounded constraints"
    //     );
    //     assert!(
    //         (a.max_height.is_finite() && b.max_height.is_finite())
    //             || (a.max_height.is_infinite() && b.max_height.is_infinite()),
    //         "Cannot interpolate between finite constraints and unbounded constraints"
    //     );

    //     const ONE: Positive<f32> = as_const!(Positive, f32, 1.0);

    //     let scalar = ONE - t;

    //     Constraints::new(
    //         if a.min_width.is_finite() && b.min_width.is_finite() {
    //             a.min_width * (ONE - t) + b.min_width * t
    //         } else {
    //             b.min_width
    //         },
    //         if a.max_width.is_finite() && b.max_width.is_finite() {
    //             a.max_width * (ONE - t) + b.max_width * t
    //         } else {
    //             b.max_width
    //         },
    //         if a.min_height.is_finite() && b.min_height.is_finite() {
    //             a.min_height * (ONE - t) + b.min_height * t
    //         } else {
    //             b.min_height
    //         },
    //         if a.max_height.is_finite() && b.max_height.is_finite() {
    //             a.max_height * (ONE - t) + b.max_height * t
    //         } else {
    //             b.max_height
    //         },
    //     )
    // }
}

impl Mul<f32> for BoxConstraints {
    type Output = BoxConstraints;

    fn mul(self, rhs: f32) -> Self::Output {
        BoxConstraints::new(
            self.min_width.get() * rhs,
            self.max_width.get() * rhs,
            self.min_height.get() * rhs,
            self.max_height.get() * rhs,
        )
    }
}

impl MulAssign<f32> for BoxConstraints {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for BoxConstraints {
    type Output = BoxConstraints;

    fn div(self, rhs: f32) -> Self::Output {
        BoxConstraints::new(
            self.min_width.get() / rhs,
            self.max_width.get() / rhs,
            self.min_height.get() / rhs,
            self.max_height.get() / rhs,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use crate::geometry::{Axis, EdgeInsets, Size};

    use super::*;

    #[test]
    fn default_is_unconstrained() {
        let c = BoxConstraints::default();
        assert_eq!(c.min_width().get(), 0.0);
        assert_eq!(c.min_height().get(), 0.0);
        assert!(c.max_width().is_infinite());
        assert!(c.max_height().is_infinite());
    }

    #[test]
    fn new_stores_bounds() {
        let c = BoxConstraints::new(10.0, 100.0, 20.0, 200.0);
        assert_eq!(c.min_width().get(), 10.0);
        assert_eq!(c.max_width().get(), 100.0);
        assert_eq!(c.min_height().get(), 20.0);
        assert_eq!(c.max_height().get(), 200.0);
    }

    #[test]
    #[should_panic(expected = "minimum width must not be greater than the maximum width")]
    fn new_panics_when_min_width_exceeds_max() {
        BoxConstraints::new(100.0, 10.0, 0.0, 100.0);
    }

    #[test]
    #[should_panic(expected = "minimum height must not be greater than the maximum height")]
    fn new_panics_when_min_height_exceeds_max() {
        BoxConstraints::new(0.0, 100.0, 100.0, 10.0);
    }

    #[test]
    fn tight_sets_min_eq_max() {
        let c = BoxConstraints::tight(Size::new(50.0, 30.0));
        assert!(c.is_tight());
        assert_eq!(c.min_width().get(), 50.0);
        assert_eq!(c.max_width().get(), 50.0);
        assert_eq!(c.min_height().get(), 30.0);
        assert_eq!(c.max_height().get(), 30.0);
    }

    #[test]
    fn loose_sets_zero_min() {
        let c = BoxConstraints::loose(Size::new(80.0, 60.0));
        assert_eq!(c.min_width().get(), 0.0);
        assert_eq!(c.max_width().get(), 80.0);
        assert_eq!(c.min_height().get(), 0.0);
        assert_eq!(c.max_height().get(), 60.0);
    }

    #[test]
    fn along_axis_horizontal() {
        let c = BoxConstraints::along_axis(Axis::Horizontal, 10.0, 50.0);
        assert_eq!(c.min_width().get(), 10.0);
        assert_eq!(c.max_width().get(), 50.0);
        assert_eq!(c.min_height().get(), 0.0);
        assert!(c.max_height().is_infinite());
    }

    #[test]
    fn along_axis_vertical() {
        let c = BoxConstraints::along_axis(Axis::Vertical, 10.0, 50.0);
        assert_eq!(c.min_width().get(), 0.0);
        assert!(c.max_width().is_infinite());
        assert_eq!(c.min_height().get(), 10.0);
        assert_eq!(c.max_height().get(), 50.0);
    }

    #[test]
    fn loosen_removes_minimums() {
        let c = BoxConstraints::new(10.0, 100.0, 20.0, 200.0).loosen();
        assert_eq!(c.min_width().get(), 0.0);
        assert_eq!(c.max_width().get(), 100.0);
        assert_eq!(c.min_height().get(), 0.0);
        assert_eq!(c.max_height().get(), 200.0);
    }

    #[test]
    fn deflate_shrinks_by_insets() {
        let c = BoxConstraints::new(40.0, 100.0, 40.0, 100.0);
        let insets = EdgeInsets::all(10.0);
        let d = c.deflate(&insets);
        // horizontal = 20, vertical = 20
        assert_eq!(d.min_width().get(), 20.0);
        assert_eq!(d.max_width().get(), 80.0);
        assert_eq!(d.min_height().get(), 20.0);
        assert_eq!(d.max_height().get(), 80.0);
    }

    #[test]
    fn deflate_clamps_to_zero() {
        let c = BoxConstraints::new(5.0, 10.0, 5.0, 10.0);
        let insets = EdgeInsets::all(20.0);
        let d = c.deflate(&insets);
        assert_eq!(d.min_width().get(), 0.0);
        assert_eq!(d.max_width().get(), 0.0);
    }

    #[test]
    fn enforce_clamps_to_other() {
        let c = BoxConstraints::new(0.0, 200.0, 0.0, 200.0);
        let bounds = BoxConstraints::new(10.0, 50.0, 20.0, 60.0);
        let e = c.enforce(bounds);
        assert_eq!(e.min_width().get(), 10.0);
        assert_eq!(e.max_width().get(), 50.0);
        assert_eq!(e.min_height().get(), 20.0);
        assert_eq!(e.max_height().get(), 60.0);
    }

    #[test]
    fn tighten_width_clamps_within_bounds() {
        let c = BoxConstraints::new(10.0, 100.0, 0.0, 100.0);
        let t = c.tighten_width(50.0);
        assert_eq!(t.min_width().get(), 50.0);
        assert_eq!(t.max_width().get(), 50.0);

        // Clamped to max
        let t = c.tighten_width(200.0);
        assert_eq!(t.min_width().get(), 100.0);
        assert_eq!(t.max_width().get(), 100.0);

        // Clamped to min
        let t = c.tighten_width(5.0);
        assert_eq!(t.min_width().get(), 10.0);
        assert_eq!(t.max_width().get(), 10.0);
    }

    #[test]
    fn tighten_height_clamps_within_bounds() {
        let c = BoxConstraints::new(0.0, 100.0, 10.0, 100.0);
        let t = c.tighten_height(50.0);
        assert_eq!(t.min_height().get(), 50.0);
        assert_eq!(t.max_height().get(), 50.0);
    }

    #[test]
    fn tighten_clamps_both_axes() {
        let c = BoxConstraints::new(0.0, 100.0, 0.0, 100.0);
        let t = c.tighten(Size::new(40.0, 60.0));
        assert!(t.is_tight());
        assert_eq!(t.min_width().get(), 40.0);
        assert_eq!(t.min_height().get(), 60.0);
    }

    #[test]
    fn constrain_clamps_size() {
        let c = BoxConstraints::new(10.0, 50.0, 10.0, 50.0);

        let s = c.constrain(Size::new(30.0, 30.0));
        assert_eq!(s.width.get(), 30.0);
        assert_eq!(s.height.get(), 30.0);

        let s = c.constrain(Size::new(0.0, 100.0));
        assert_eq!(s.width.get(), 10.0);
        assert_eq!(s.height.get(), 50.0);
    }

    #[test]
    fn constrain_preserve_aspect_ratio_scales_down() {
        let c = BoxConstraints::new(0.0, 100.0, 0.0, 50.0);
        let s = c.constrain_preserve_aspect_ratio(Size::new(200.0, 100.0));
        assert_eq!(s.width.get(), 100.0);
        assert_eq!(s.height.get(), 50.0);
    }

    #[test]
    fn constrain_preserve_aspect_ratio_scales_up_to_min() {
        let c = BoxConstraints::new(100.0, 200.0, 50.0, 100.0);
        let s = c.constrain_preserve_aspect_ratio(Size::new(20.0, 10.0));
        assert_eq!(s.width.get(), 100.0);
        assert_eq!(s.height.get(), 50.0);
    }

    #[test]
    fn flip_swaps_axes() {
        let c = BoxConstraints::new(10.0, 100.0, 20.0, 200.0);
        let f = c.flip();
        assert_eq!(f.min_width().get(), 20.0);
        assert_eq!(f.max_width().get(), 200.0);
        assert_eq!(f.min_height().get(), 10.0);
        assert_eq!(f.max_height().get(), 100.0);
    }

    #[test]
    fn smallest_and_biggest() {
        let c = BoxConstraints::new(10.0, 100.0, 20.0, 200.0);
        let s = c.smallest();
        assert_eq!(s.width.get(), 10.0);
        assert_eq!(s.height.get(), 20.0);
        let b = c.biggest();
        assert_eq!(b.width.get(), 100.0);
        assert_eq!(b.height.get(), 200.0);
    }

    #[test]
    fn has_tight_and_bounded_queries() {
        let tight = BoxConstraints::tight(Size::new(10.0, 20.0));
        assert!(tight.has_tight_width());
        assert!(tight.has_tight_height());
        assert!(tight.is_tight());
        assert!(tight.has_bounded_width());
        assert!(tight.has_bounded_height());
        assert!(!tight.has_infinite_width());
        assert!(!tight.has_infinite_height());

        let expand = BoxConstraints::expand();
        assert!(!expand.has_tight_width());
        assert!(!expand.has_tight_height());
        assert!(!expand.has_bounded_width());
        assert!(expand.has_infinite_width());
        assert!(expand.has_infinite_height());
    }

    #[test]
    fn is_satisfied_by() {
        let c = BoxConstraints::new(10.0, 50.0, 10.0, 50.0);
        assert!(c.is_satisfied_by(Size::new(30.0, 30.0)));
        assert!(c.is_satisfied_by(Size::new(10.0, 50.0)));
        assert!(!c.is_satisfied_by(Size::new(5.0, 30.0)));
        assert!(!c.is_satisfied_by(Size::new(30.0, 60.0)));
    }

    #[test]
    fn only_width_and_only_height() {
        let c = BoxConstraints::new(10.0, 50.0, 20.0, 60.0);

        let w = c.only_width();
        assert_eq!(w.min_width().get(), 10.0);
        assert_eq!(w.max_width().get(), 50.0);
        assert_eq!(w.min_height().get(), 0.0);
        assert!(w.max_height().is_infinite());

        let h = c.only_height();
        assert_eq!(h.min_width().get(), 0.0);
        assert!(h.max_width().is_infinite());
        assert_eq!(h.min_height().get(), 20.0);
        assert_eq!(h.max_height().get(), 60.0);
    }

    #[test]
    fn mul_scales_all_bounds() {
        let c = BoxConstraints::new(10.0, 100.0, 20.0, 200.0);
        let scaled = c * 2.0;
        assert_eq!(scaled.min_width().get(), 20.0);
        assert_eq!(scaled.max_width().get(), 200.0);
        assert_eq!(scaled.min_height().get(), 40.0);
        assert_eq!(scaled.max_height().get(), 400.0);
    }

    #[test]
    fn div_scales_all_bounds() {
        let c = BoxConstraints::new(10.0, 100.0, 20.0, 200.0);
        let scaled = c / 2.0;
        assert_eq!(scaled.min_width().get(), 5.0);
        assert_eq!(scaled.max_width().get(), 50.0);
        assert_eq!(scaled.min_height().get(), 10.0);
        assert_eq!(scaled.max_height().get(), 100.0);
    }

    #[test]
    fn debug_prints_ranges_per_axis() {
        let c = BoxConstraints::new(0.0, 300.0, 0.0, 200.0);
        assert_eq!(format!("{c:?}"), "0.0<=w<=300.0, 0.0<=h<=200.0");
    }

    #[test]
    fn debug_collapses_a_tight_axis() {
        let c = BoxConstraints::new(100.0, 100.0, 0.0, 200.0);
        assert_eq!(format!("{c:?}"), "w=100.0, 0.0<=h<=200.0");
    }

    #[test]
    fn debug_prints_infinite_max_as_inf() {
        let c = BoxConstraints::along_axis(Axis::Horizontal, 0.0, 300.0);
        assert_eq!(format!("{c:?}"), "0.0<=w<=300.0, 0.0<=h<=inf");
    }
}
