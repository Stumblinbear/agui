use std::ops::{Div, Mul, MulAssign};

use typed_floats::{as_const, Positive, StrictlyPositiveFinite};

use crate::{axis::Axis, edge_insets::EdgeInsetsGeometry, size::Size};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Constraints {
    min_width: Positive<f32>,
    max_width: Positive<f32>,
    min_height: Positive<f32>,
    max_height: Positive<f32>,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min_width: as_const!(Positive, f32, 0.0),
            max_width: as_const!(Positive, f32, f32::INFINITY),
            min_height: as_const!(Positive, f32, 0.0),
            max_height: as_const!(Positive, f32, f32::INFINITY),
        }
    }
}

impl Constraints {
    pub fn new<T>(min_width: T, max_width: T, min_height: T, max_height: T) -> Self
    where
        Positive<f32>: TryFrom<T>,
        <Positive<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        let min_width =
            Positive::try_from(min_width).expect("mininimum width must be a non-negative number");
        let max_width =
            Positive::try_from(max_width).expect("maxinimum width must be a non-negative number");
        let min_height =
            Positive::try_from(min_height).expect("mininimum height must be a non-negative number");
        let max_height =
            Positive::try_from(max_height).expect("maxinimum height must be a non-negative number");

        assert!(
            min_width <= max_width,
            "mininimum width must not be greater than the maximum width"
        );

        assert!(
            min_height <= max_height,
            "mininimum height must not be greater than the maximum height"
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
    pub fn along_axis(axis: Axis, min: f32, max: f32) -> Self {
        let min =
            Positive::try_from(min).expect("mininimum constraint must be a non-negative number");
        let max =
            Positive::try_from(max).expect("maxinimum constraint must be a non-negative number");

        assert!(
            min <= max,
            "mininimum constraint must not be greater than the maximum constraint"
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

    /// Creates [`Constraints`] that require the given size on the given axis.
    pub fn tight_for(axis: Axis, size: f32) -> Self {
        Self::along_axis(axis, size, size)
    }

    /// Creates [`Constraints`] that forbids sizes larger than the given size.
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
    pub fn loose_for(axis: Axis, size: f32) -> Self {
        Self::along_axis(axis, 0.0, size)
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

        // SAFETY: the deflated constraints are guaranteed to be non-negative
        unsafe {
            Self {
                min_width: Positive::<f32>::new_unchecked(deflated_min_width.get()),
                max_width: Positive::<f32>::new_unchecked(deflated_max_width.get()),
                min_height: Positive::<f32>::new_unchecked(deflated_min_height.get()),
                max_height: Positive::<f32>::new_unchecked(deflated_max_height.get()),
            }
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
    pub fn enforce(self, other: impl Into<Constraints>) -> Self {
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

        let width = other.width.get();
        let height = other.height.get();

        // SAFETY: since [`Size`] is guaranteed to not be NaN, clamp will always return positive numbers
        unsafe {
            Self {
                min_width: Positive::<f32>::new_unchecked(
                    width.clamp(self.min_width.get(), self.max_width.get()),
                ),
                max_width: Positive::<f32>::new_unchecked(
                    width.clamp(self.min_width.get(), self.max_width.get()),
                ),
                min_height: Positive::<f32>::new_unchecked(
                    height.clamp(self.min_height.get(), self.max_height.get()),
                ),
                max_height: Positive::<f32>::new_unchecked(
                    height.clamp(self.min_height.get(), self.max_height.get()),
                ),
            }
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
    pub fn constrain_width(self, width: f32) -> Positive<f32> {
        let width =
            Positive::try_from(width).expect("constrained width must be a non-negative number");

        width.clamp(self.min_width, self.max_width)
    }

    /// Returns the height that both satisfies the constraints and is as close as
    /// possible to the given height.
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

impl Mul<f32> for Constraints {
    type Output = Constraints;

    fn mul(self, rhs: f32) -> Self::Output {
        Constraints::new(
            self.min_width.get() * rhs,
            self.max_width.get() * rhs,
            self.min_height.get() * rhs,
            self.max_height.get() * rhs,
        )
    }
}

impl MulAssign<f32> for Constraints {
    fn mul_assign(&mut self, rhs: f32) {
        *self = *self * rhs;
    }
}

impl Div<f32> for Constraints {
    type Output = Constraints;

    fn div(self, rhs: f32) -> Self::Output {
        Constraints::new(
            self.min_width.get() / rhs,
            self.max_width.get() / rhs,
            self.min_height.get() / rhs,
            self.max_height.get() / rhs,
        )
    }
}
