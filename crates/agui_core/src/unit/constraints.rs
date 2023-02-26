use std::ops::{Div, DivAssign, Mul, MulAssign, Rem, RemAssign};

use super::{Axis, EdgeInsets, Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }
}

impl Constraints {
    pub fn new(min_width: f32, max_width: f32, min_height: f32, max_height: f32) -> Self {
        assert!(min_width <= max_width);
        assert!(min_height <= max_height);

        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }

    /// Creates [`Constraints`] that require the given size.
    pub fn tight(size: Size) -> Self {
        Self {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
        }
    }

    /// Creates [`Constraints`] that require the given size on the given axis.
    pub fn tight_for(axis: Axis, size: f32) -> Self {
        match axis {
            Axis::Horizontal => Self {
                min_width: size,
                max_width: size,
                ..Default::default()
            },
            Axis::Vertical => Self {
                min_height: size,
                max_height: size,
                ..Default::default()
            },
        }
    }

    /// Creates [`Constraints`] that forbids sizes larger than the given size.
    pub fn loose(size: Size) -> Self {
        Self {
            max_width: size.width,
            max_height: size.height,
            ..Default::default()
        }
    }

    /// Creates [`Constraints`] that forbids sizes larger than the given size on the given axis.
    pub fn loose_for(axis: Axis, size: f32) -> Self {
        match axis {
            Axis::Horizontal => Self {
                max_width: size,
                ..Default::default()
            },
            Axis::Vertical => Self {
                max_height: size,
                ..Default::default()
            },
        }
    }

    pub fn expand() -> Self {
        Self {
            min_width: f32::INFINITY,
            max_width: f32::INFINITY,
            min_height: f32::INFINITY,
            max_height: f32::INFINITY,
        }
    }

    pub fn width(&self, width: f32) -> f32 {
        width.clamp(self.min_width, self.max_width)
    }

    pub fn height(&self, height: f32) -> f32 {
        height.clamp(self.min_height, self.max_height)
    }

    pub fn size(&self, size: impl Into<Size>) -> Size {
        let size = size.into();

        Size {
            width: self.width(size.width),
            height: self.height(size.height),
        }
    }

    /// Returns new [`Constraints`] that are smaller by the given edge dimensions.
    pub fn deflate(self, insets: impl Into<EdgeInsets>) -> Self {
        let insets = insets.into();

        let horizontal_size = insets.horizontal();
        let vertical_size = insets.vertical();

        let deflated_min_width = 0.0_f32.max(self.min_width - horizontal_size);
        let deflated_min_height = 0.0_f32.max(self.min_height - vertical_size);
        let deflated_max_width = deflated_min_width.max(self.max_width - horizontal_size);
        let deflated_max_height = deflated_min_height.max(self.max_height - vertical_size);

        Self {
            min_width: deflated_min_width,
            min_height: deflated_min_height,
            max_width: deflated_max_width,
            max_height: deflated_max_height,
        }
    }

    /// Returns new [`Constraints`] that remove the minimum width and height requirements.
    pub fn loosen(self) -> Self {
        Self {
            min_width: 0.0,
            min_height: 0.0,
            max_width: self.max_width,
            max_height: self.max_height,
        }
    }

    /// Returns new [`Constraints`] that respect the given constraints while being as
    /// close as possible to the original constraints.
    pub fn enforce(self, other: impl Into<Constraints>) -> Self {
        let other = other.into();

        Self {
            min_width: self.min_width.clamp(other.min_width, other.max_width),
            min_height: self.min_height.clamp(other.min_height, other.max_height),
            max_width: self.max_width.clamp(other.min_width, other.max_width),
            max_height: self.max_height.clamp(other.min_height, other.max_height),
        }
    }

    /// Returns new [`Constraints`] with a tight width and/or height as close to the given
    /// width and height as possible while still respecting the original constraints.
    pub fn tighten(self, other: impl Into<Constraints>) -> Self {
        let other = other.into();

        Self {
            min_width: other.min_width.clamp(self.min_width, other.max_width),
            min_height: other.min_height.clamp(self.min_height, other.max_height),
            max_width: other.max_width.clamp(self.min_width, other.max_width),
            max_height: other.max_height.clamp(self.min_height, other.max_height),
        }
    }

    /// Returns new [`Constraints`] with the width and height constraints flipped.
    pub fn flip(self) -> Self {
        Self {
            min_width: self.min_height,
            min_height: self.min_width,
            max_width: self.max_height,
            max_height: self.max_width,
        }
    }

    /// Returns new [`Constraints`] with the same width constraints but with unconstrained
    /// height.
    pub fn only_width(&self) -> Self {
        Self {
            min_width: self.min_width,
            max_width: self.max_width,
            ..Default::default()
        }
    }

    /// Returns new [`Constraints`] with the same height constraints but with unconstrained
    /// width.
    pub fn only_height(&self) -> Self {
        Self {
            min_height: self.min_height,
            max_height: self.max_height,
            ..Default::default()
        }
    }

    /// Returns the width that both satisfies the constraints and is as close as
    /// possible to the given width.
    pub fn constrain_width(&self, width: f32) -> f32 {
        width.clamp(self.min_width, self.max_width)
    }

    /// Returns the height that both satisfies the constraints and is as close as
    /// possible to the given height.
    pub fn constrain_height(&self, height: f32) -> f32 {
        height.clamp(self.min_height, self.max_height)
    }

    /// Returns the [Size] that both satisfies the constraints and is as close as
    /// possible to the given size.
    pub fn constrain(&self, size: impl Into<Size>) -> Size {
        let size = size.into();

        Size {
            width: self.constrain_width(size.width),
            height: self.constrain_height(size.height),
        }
    }

    /// Returns a [Size] that attempts to meet the following conditions, in order:
    ///
    ///  * The size must satisfy these constraints.
    ///  * The aspect ratio of the returned size matches the aspect ratio of the
    ///    given size.
    ///  * The returned size as big as possible while still being equal to or
    ///    smaller than the given size.
    pub fn constrain_preserve_aspect_ratio(&self, size: impl Into<Size>) -> Size {
        let size = size.into();

        let mut width = size.width;
        let mut height = size.height;

        let aspect_ratio = size.width / size.height;

        if width > self.max_width {
            width = self.max_width;
            height = width / aspect_ratio;
        }

        if height > self.max_height {
            height = self.max_height;
            width = height * aspect_ratio;
        }

        if width < self.min_width {
            width = self.min_width;
            height = width / aspect_ratio;
        }

        if height < self.min_height {
            height = self.min_height;
            width = height * aspect_ratio;
        }

        Size {
            width: self.constrain_width(width),
            height: self.constrain_height(height),
        }
    }

    /// The smallest [Size] that satisfies the constraints.
    pub fn smallest(&self) -> Size {
        Size {
            width: self.min_width,
            height: self.min_height,
        }
    }

    /// The biggest [Size] that satisfies the constraints.
    pub fn biggest(&self) -> Size {
        Size {
            width: self.max_width,
            height: self.max_height,
        }
    }

    /// Whether there is exactly one width value that satisfies the constraints.
    pub fn has_tight_width(&self) -> bool {
        self.min_width == self.max_width
    }

    /// Whether there is exactly one height value that satisfies the constraints
    pub fn has_tight_height(&self) -> bool {
        self.min_height == self.max_height
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

    pub fn get_extent_min(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.min_width,
            Axis::Vertical => self.min_height,
        }
    }

    pub fn get_extent_max(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.max_width,
            Axis::Vertical => self.max_height,
        }
    }

    pub fn lerp(a: &Constraints, b: &Constraints, t: f32) -> Constraints {
        assert!(
            (a.min_width.is_finite() && b.min_width.is_finite())
                || (a.min_width.is_infinite() && b.min_width.is_infinite()),
            "Cannot interpolate between finite constraints and unbounded constraints"
        );
        assert!(
            (a.max_width.is_finite() && b.max_width.is_finite())
                || (a.max_width.is_infinite() && b.max_width.is_infinite()),
            "Cannot interpolate between finite constraints and unbounded constraints"
        );

        assert!(
            (a.min_height.is_finite() && b.min_height.is_finite())
                || (a.min_height.is_infinite() && b.min_height.is_infinite()),
            "Cannot interpolate between finite constraints and unbounded constraints"
        );
        assert!(
            (a.max_height.is_finite() && b.max_height.is_finite())
                || (a.max_height.is_infinite() && b.max_height.is_infinite()),
            "Cannot interpolate between finite constraints and unbounded constraints"
        );

        Constraints {
            min_width: if a.min_width.is_finite() {
                a.min_width * (1.0 - t) + b.min_width * t
            } else {
                f32::INFINITY
            },

            max_width: if a.max_width.is_finite() {
                a.max_width * (1.0 - t) + b.max_width * t
            } else {
                f32::INFINITY
            },

            min_height: if a.min_height.is_finite() {
                a.min_height * (1.0 - t) + b.min_height * t
            } else {
                f32::INFINITY
            },

            max_height: if a.max_height.is_finite() {
                a.max_height * (1.0 - t) + b.max_height * t
            } else {
                f32::INFINITY
            },
        }
    }
}

impl Mul<f32> for Constraints {
    type Output = Constraints;

    fn mul(self, factor: f32) -> Self::Output {
        Constraints {
            min_width: self.min_width * factor,
            max_width: self.max_width * factor,
            min_height: self.min_height * factor,
            max_height: self.max_height * factor,
        }
    }
}

impl MulAssign<Constraints> for Constraints {
    fn mul_assign(&mut self, other: Constraints) {
        self.min_width *= other.min_width;
        self.max_width *= other.max_width;
        self.min_height *= other.min_height;
        self.max_height *= other.max_height;
    }
}

impl Div<f32> for Constraints {
    type Output = Constraints;

    fn div(self, factor: f32) -> Self::Output {
        Constraints {
            min_width: self.min_width / factor,
            max_width: self.max_width / factor,
            min_height: self.min_height / factor,
            max_height: self.max_height / factor,
        }
    }
}

impl DivAssign<Constraints> for Constraints {
    fn div_assign(&mut self, other: Constraints) {
        self.min_width /= other.min_width;
        self.max_width /= other.max_width;
        self.min_height /= other.min_height;
        self.max_height /= other.max_height;
    }
}

impl Rem<f32> for Constraints {
    type Output = Constraints;

    fn rem(self, factor: f32) -> Self::Output {
        Constraints {
            min_width: self.min_width % factor,
            max_width: self.max_width % factor,
            min_height: self.min_height % factor,
            max_height: self.max_height % factor,
        }
    }
}

impl RemAssign<Constraints> for Constraints {
    fn rem_assign(&mut self, other: Constraints) {
        self.min_width %= other.min_width;
        self.max_width %= other.max_width;
        self.min_height %= other.min_height;
        self.max_height %= other.max_height;
    }
}

impl From<Size> for Constraints {
    fn from(size: Size) -> Self {
        Constraints {
            min_width: size.width,
            max_width: size.width,
            min_height: size.height,
            max_height: size.height,
        }
    }
}
