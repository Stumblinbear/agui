use typed_floats::{NonNaNFinite, as_const};

use crate::{offset::Offset, rect::Rect, size::Size};

/// A point within a box, named as a fraction of its size.
///
/// `x` runs from `-1.0` at the left edge to `1.0` at the right, and `y` from `-1.0` at the top to
/// `1.0` at the bottom, so `(0.0, 0.0)` is the center. Values outside `[-1.0, 1.0]` name points
/// outside the box.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Alignment {
    pub x: NonNaNFinite<f32>,
    pub y: NonNaNFinite<f32>,
}

impl Alignment {
    pub const TOP_LEFT: Self = Self {
        x: as_const!(NonNaNFinite, f32, -1.0),
        y: as_const!(NonNaNFinite, f32, -1.0),
    };
    pub const TOP_CENTER: Self = Self {
        x: as_const!(NonNaNFinite, f32, 0.0),
        y: as_const!(NonNaNFinite, f32, -1.0),
    };
    pub const TOP_RIGHT: Self = Self {
        x: as_const!(NonNaNFinite, f32, 1.0),
        y: as_const!(NonNaNFinite, f32, -1.0),
    };
    pub const CENTER_LEFT: Self = Self {
        x: as_const!(NonNaNFinite, f32, -1.0),
        y: as_const!(NonNaNFinite, f32, 0.0),
    };
    pub const CENTER: Self = Self {
        x: as_const!(NonNaNFinite, f32, 0.0),
        y: as_const!(NonNaNFinite, f32, 0.0),
    };
    pub const CENTER_RIGHT: Self = Self {
        x: as_const!(NonNaNFinite, f32, 1.0),
        y: as_const!(NonNaNFinite, f32, 0.0),
    };
    pub const BOTTOM_LEFT: Self = Self {
        x: as_const!(NonNaNFinite, f32, -1.0),
        y: as_const!(NonNaNFinite, f32, 1.0),
    };
    pub const BOTTOM_CENTER: Self = Self {
        x: as_const!(NonNaNFinite, f32, 0.0),
        y: as_const!(NonNaNFinite, f32, 1.0),
    };
    pub const BOTTOM_RIGHT: Self = Self {
        x: as_const!(NonNaNFinite, f32, 1.0),
        y: as_const!(NonNaNFinite, f32, 1.0),
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

    /// The point this alignment names within a box of `size`, measured from the box's top-left.
    pub fn along_size(self, size: Size) -> Offset {
        let half_width = size.width.get() / 2.0;
        let half_height = size.height.get() / 2.0;

        Offset::new(
            half_width + self.x.get() * half_width,
            half_height + self.y.get() * half_height,
        )
    }

    /// The point this alignment names within a box of the given extent, measured from its top-left.
    /// `extent` is read as a width and height.
    pub fn along_offset(self, extent: Offset) -> Offset {
        let half_width = extent.x.get() / 2.0;
        let half_height = extent.y.get() / 2.0;

        Offset::new(
            half_width + self.x.get() * half_width,
            half_height + self.y.get() * half_height,
        )
    }

    /// The point this alignment names within `rect`, in the same coordinate space as `rect`.
    pub fn within_rect(self, rect: Rect) -> Offset {
        let half_width = rect.width.get() / 2.0;
        let half_height = rect.height.get() / 2.0;

        Offset::new(
            rect.left.get() + half_width + self.x.get() * half_width,
            rect.top.get() + half_height + self.y.get() * half_height,
        )
    }

    /// Places a box of `size` inside `rect` at this alignment, returning where it lands.
    pub fn inscribe(self, size: Size, rect: Rect) -> Rect {
        let half_width_delta = (rect.width.get() - size.width.get()) / 2.0;
        let half_height_delta = (rect.height.get() - size.height.get()) / 2.0;

        let origin = Offset::new(
            rect.left.get() + half_width_delta + self.x.get() * half_width_delta,
            rect.top.get() + half_height_delta + self.y.get() * half_height_delta,
        );

        Rect::new(origin.x, origin.y, size.width, size.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn along_size_names_a_point_in_the_box() {
        let size = Size::new(100.0, 40.0);

        assert_eq!(Alignment::TOP_LEFT.along_size(size), Offset::new(0.0, 0.0));
        assert_eq!(Alignment::CENTER.along_size(size), Offset::new(50.0, 20.0));
        assert_eq!(
            Alignment::BOTTOM_RIGHT.along_size(size),
            Offset::new(100.0, 40.0)
        );
        assert_eq!(
            Alignment::TOP_CENTER.along_size(size),
            Offset::new(50.0, 0.0)
        );
    }

    #[test]
    fn a_fraction_outside_the_unit_range_lands_outside_the_box() {
        let size = Size::new(100.0, 100.0);

        assert_eq!(
            Alignment::new(2.0, -2.0).along_size(size),
            Offset::new(150.0, -50.0)
        );
    }

    #[test]
    fn within_rect_is_relative_to_the_rect_origin() {
        let rect = Rect::new(10.0, 20.0, 100.0, 40.0);

        assert_eq!(Alignment::CENTER.within_rect(rect), Offset::new(60.0, 40.0));
        assert_eq!(
            Alignment::TOP_LEFT.within_rect(rect),
            Offset::new(10.0, 20.0)
        );
    }

    #[test]
    fn inscribe_places_a_smaller_box_by_its_alignment() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let size = Size::new(20.0, 20.0);

        assert_eq!(
            Alignment::CENTER.inscribe(size, rect),
            Rect::new(40.0, 40.0, 20.0, 20.0)
        );
        assert_eq!(
            Alignment::TOP_LEFT.inscribe(size, rect),
            Rect::new(0.0, 0.0, 20.0, 20.0)
        );
        assert_eq!(
            Alignment::BOTTOM_RIGHT.inscribe(size, rect),
            Rect::new(80.0, 80.0, 20.0, 20.0)
        );
    }
}
