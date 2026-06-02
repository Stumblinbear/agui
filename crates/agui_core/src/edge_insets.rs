use std::ops::{Add, Sub};

use typed_floats::{Max, NonNaNFinite, Positive, PositiveFinite, as_const};

use crate::{axis::Axis, offset::Offset, size::Size, text_direction::TextDirection};

pub trait EdgeInsetsGeometry {
    fn is_zero(&self) -> bool;

    fn top(&self) -> PositiveFinite<f32>;

    fn right(&self, text_direction: TextDirection) -> PositiveFinite<f32>;

    fn bottom(&self) -> PositiveFinite<f32>;

    fn left(&self, text_direction: TextDirection) -> PositiveFinite<f32>;

    fn horizontal(&self) -> PositiveFinite<f32>;

    fn vertical(&self) -> PositiveFinite<f32>;

    fn axis(&self, axis: Axis) -> PositiveFinite<f32> {
        match axis {
            Axis::Horizontal => self.horizontal(),
            Axis::Vertical => self.vertical(),
        }
    }

    /// The total inset along each axis, as a [`Size`] (horizontal as width, vertical as height).
    fn collapsed_size(&self) -> Size {
        Size::new(self.horizontal(), self.vertical())
    }

    /// Grows `size` by the insets on each axis.
    fn inflate_size(&self, size: Size) -> Size {
        size + self.collapsed_size()
    }

    /// Shrinks `size` by the insets on each axis.
    fn deflate_size(&self, size: Size) -> Size {
        size - self.collapsed_size()
    }

    fn inflate_axis(&self, axis: Axis, extent: Positive<f32>) -> Positive<f32> {
        extent + self.axis(axis)
    }

    fn deflate_axis(&self, axis: Axis, extent: Positive<f32>) -> Positive<f32> {
        let deflated = Max::max(
            extent - self.axis(axis),
            as_const!(PositiveFinite, f32, 0.0),
        );

        // We have to call abs() due to the possibility of -0.0 being produced.
        deflated.abs()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EdgeInsets {
    pub top: PositiveFinite<f32>,
    pub right: PositiveFinite<f32>,
    pub bottom: PositiveFinite<f32>,
    pub left: PositiveFinite<f32>,
}

impl Default for EdgeInsets {
    fn default() -> Self {
        Self::ZERO
    }
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        top: as_const!(PositiveFinite, f32, 0.0),
        right: as_const!(PositiveFinite, f32, 0.0),
        bottom: as_const!(PositiveFinite, f32, 0.0),
        left: as_const!(PositiveFinite, f32, 0.0),
    };

    /// # Panics
    ///
    /// Panics if any inset is negative, infinite, or NaN.
    pub fn new<T>(top: T, right: T, bottom: T, left: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self {
            top: PositiveFinite::try_from(top).expect("top must be a positive finite number"),
            right: PositiveFinite::try_from(right).expect("right must a positive finite number"),
            bottom: PositiveFinite::try_from(bottom).expect("bottom must a positive finite number"),
            left: PositiveFinite::try_from(left).expect("left must a positive finite number"),
        }
    }

    /// # Panics
    ///
    /// Panics if `value` is negative, infinite, or NaN.
    pub fn all<T>(value: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        let value =
            PositiveFinite::try_from(value).expect("value must be a positive finite number");

        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// # Panics
    ///
    /// Panics if `vertical` or `horizontal` is negative, infinite, or NaN.
    pub fn symmetric<T>(vertical: T, horizontal: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        let vertical =
            PositiveFinite::try_from(vertical).expect("vertical must be a positive finite number");
        let horizontal = PositiveFinite::try_from(horizontal)
            .expect("horizontal must be a positive finite number");

        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// The offset from the box's top-left corner to the content's top-left corner.
    pub fn top_left(&self) -> Offset {
        Offset {
            x: self.left.into(),
            y: self.top.into(),
        }
    }

    /// The offset from the box's top-right corner to the content's top-right corner.
    pub fn top_right(&self) -> Offset {
        Offset {
            x: -NonNaNFinite::from(self.right),
            y: self.top.into(),
        }
    }

    /// The offset from the box's bottom-left corner to the content's bottom-left corner.
    pub fn bottom_left(&self) -> Offset {
        Offset {
            x: self.left.into(),
            y: -NonNaNFinite::from(self.bottom),
        }
    }

    /// The offset from the box's bottom-right corner to the content's bottom-right corner.
    pub fn bottom_right(&self) -> Offset {
        Offset {
            x: -NonNaNFinite::from(self.right),
            y: -NonNaNFinite::from(self.bottom),
        }
    }

    /// Swaps the top/bottom and left/right insets.
    pub fn flipped(&self) -> Self {
        Self {
            top: self.bottom,
            right: self.left,
            bottom: self.top,
            left: self.right,
        }
    }
}

impl EdgeInsetsGeometry for EdgeInsets {
    fn is_zero(&self) -> bool {
        self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0 && self.left == 0.0
    }

    fn top(&self) -> PositiveFinite<f32> {
        self.top
    }

    fn right(&self, _: TextDirection) -> PositiveFinite<f32> {
        self.right
    }

    fn bottom(&self) -> PositiveFinite<f32> {
        self.bottom
    }

    fn left(&self, _: TextDirection) -> PositiveFinite<f32> {
        self.left
    }

    fn horizontal(&self) -> PositiveFinite<f32> {
        PositiveFinite::try_from(self.left + self.right)
            .expect("total horizontal edge insets must be a positive finite number")
    }

    fn vertical(&self) -> PositiveFinite<f32> {
        PositiveFinite::try_from(self.top + self.bottom)
            .expect("total vertical edge insets must be a positive finite number")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct DirectionalEdgeInsets {
    pub start: PositiveFinite<f32>,
    pub top: PositiveFinite<f32>,
    pub end: PositiveFinite<f32>,
    pub bottom: PositiveFinite<f32>,
}

impl DirectionalEdgeInsets {
    pub const ZERO: Self = Self {
        start: as_const!(PositiveFinite, f32, 0.0),
        top: as_const!(PositiveFinite, f32, 0.0),
        end: as_const!(PositiveFinite, f32, 0.0),
        bottom: as_const!(PositiveFinite, f32, 0.0),
    };

    /// # Panics
    ///
    /// Panics if any inset is negative, infinite, or NaN.
    pub fn new<T>(start: T, top: T, end: T, bottom: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self {
            start: PositiveFinite::try_from(start).expect("start must be a positive finite number"),
            top: PositiveFinite::try_from(top).expect("top must a positive finite number"),
            end: PositiveFinite::try_from(end).expect("end must a positive finite number"),
            bottom: PositiveFinite::try_from(bottom).expect("bottom must a positive finite number"),
        }
    }

    /// # Panics
    ///
    /// Panics if `start + end` overflows to infinity.
    pub fn horizontal(&self) -> PositiveFinite<f32> {
        PositiveFinite::try_from(self.start + self.end)
            .expect("total horizontal edge insets must be a positive finite number")
    }

    /// # Panics
    ///
    /// Panics if `top + bottom` overflows to infinity.
    pub fn vertical(&self) -> PositiveFinite<f32> {
        PositiveFinite::try_from(self.top + self.bottom)
            .expect("total vertical edge insets must be a positive finite number")
    }

    pub fn axis(&self, axis: Axis) -> PositiveFinite<f32> {
        match axis {
            Axis::Horizontal => self.horizontal(),
            Axis::Vertical => self.vertical(),
        }
    }
}

impl EdgeInsetsGeometry for DirectionalEdgeInsets {
    fn is_zero(&self) -> bool {
        self.start == 0.0 && self.top == 0.0 && self.end == 0.0 && self.bottom == 0.0
    }

    fn top(&self) -> PositiveFinite<f32> {
        self.top
    }

    fn right(&self, text_direction: TextDirection) -> PositiveFinite<f32> {
        match text_direction {
            TextDirection::LeftToRight => self.end,
            TextDirection::RightToLeft => self.start,
        }
    }

    fn bottom(&self) -> PositiveFinite<f32> {
        self.bottom
    }

    fn left(&self, text_direction: TextDirection) -> PositiveFinite<f32> {
        match text_direction {
            TextDirection::LeftToRight => self.start,
            TextDirection::RightToLeft => self.end,
        }
    }

    fn horizontal(&self) -> PositiveFinite<f32> {
        PositiveFinite::try_from(self.start + self.end)
            .expect("total horizontal edge insets must be a positive finite number")
    }

    fn vertical(&self) -> PositiveFinite<f32> {
        PositiveFinite::try_from(self.top + self.bottom)
            .expect("total vertical edge insets must be a positive finite number")
    }
}
