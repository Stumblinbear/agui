use std::fmt;
use std::ops::Add;

use peniko::kurbo::{Affine, Point};
use typed_floats::{NonNaN, NonNaNFinite, as_const};

use crate::geometry::{Offset, Size};

/// Holds exact position and size values.
#[derive(Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Rect {
    pub left: NonNaNFinite<f32>,
    pub top: NonNaNFinite<f32>,
    pub width: NonNaN<f32>,
    pub height: NonNaN<f32>,
}

impl fmt::Debug for Rect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rect")
            .field("left", &self.left.get())
            .field("top", &self.top.get())
            .field("width", &self.width.get())
            .field("height", &self.height.get())
            .finish()
    }
}

impl Rect {
    /// # Panics
    ///
    /// Panics if `left` or `top` is infinite or NaN, or if `width` or `height` is NaN.
    pub fn new<P, S>(left: P, top: P, width: S, height: S) -> Self
    where
        NonNaNFinite<f32>: TryFrom<P>,
        <NonNaNFinite<f32> as TryFrom<P>>::Error: std::fmt::Debug,
        NonNaN<f32>: TryFrom<S>,
        <NonNaN<f32> as TryFrom<S>>::Error: std::fmt::Debug,
    {
        Self {
            left: NonNaNFinite::try_from(left).expect("left must be a finite number"),
            top: NonNaNFinite::try_from(top).expect("top must be a finite number"),
            width: NonNaN::try_from(width).expect("width must not be NaN"),
            height: NonNaN::try_from(height).expect("height must not be NaN"),
        }
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.left && point.0 <= self.left + self.width)
            && (point.1 >= self.top && point.1 <= self.top + self.height)
    }

    /// The top edge, `top`.
    pub const fn top(&self) -> NonNaNFinite<f32> {
        self.top
    }

    /// The right edge, `left + width`.
    pub fn right(&self) -> NonNaN<f32> {
        self.left + self.width
    }

    /// The bottom edge, `top + height`.
    pub fn bottom(&self) -> NonNaN<f32> {
        self.top + self.height
    }

    /// The left edge, `left`.
    pub const fn left(&self) -> NonNaNFinite<f32> {
        self.left
    }

    /// The width, `width`.
    pub const fn width(&self) -> NonNaN<f32> {
        self.width
    }

    /// The height, `height`.
    pub const fn height(&self) -> NonNaN<f32> {
        self.height
    }

    /// Whether this rect and `other` overlap.
    pub fn intersects(&self, other: Rect) -> bool {
        self.left.get() < other.right()
            && other.left.get() < self.right()
            && self.top.get() < other.bottom()
            && other.top.get() < self.bottom()
    }

    /// The overlap of this rect and `other`, or an empty rect when they do not overlap.
    pub fn intersect(&self, other: Rect) -> Rect {
        let left = self.left.max(other.left);
        let top = self.top.max(other.top);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        Rect::new(
            left,
            top,
            (right - left).max(as_const!(NonNaN, f32, 0.0)),
            (bottom - top).max(as_const!(NonNaN, f32, 0.0)),
        )
    }

    /// The axis-aligned bounding box of this rect after `transform`.
    #[allow(clippy::cast_possible_truncation)]
    pub fn transform_bbox(&self, transform: Affine) -> Rect {
        let left = f64::from(self.left.get());
        let top = f64::from(self.top.get());
        let right = f64::from(self.right().get());
        let bottom = f64::from(self.bottom().get());

        let corners = [
            transform * Point::new(left, top),
            transform * Point::new(right, top),
            transform * Point::new(left, bottom),
            transform * Point::new(right, bottom),
        ];

        let min_x = corners.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
        let min_y = corners.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
        let max_x = corners
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_y = corners
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max);

        Rect::new(
            min_x as f32,
            min_y as f32,
            (max_x - min_x) as f32,
            (max_y - min_y) as f32,
        )
    }

    /// The smallest rect containing both this rect and `other`.
    pub fn union(&self, other: Rect) -> Rect {
        let left = self.left.min(other.left);
        let top = self.top.min(other.top);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());

        Rect::new(left, top, right - left, bottom - top)
    }

    /// The top-left corner.
    pub fn origin(&self) -> Offset {
        Offset::from((self.left.get(), self.top.get()))
    }

    /// The width and height.
    pub fn size(&self) -> Size {
        Size::new(self.width.get(), self.height.get())
    }
}

impl Add<Offset> for Rect {
    type Output = Rect;

    fn add(self, offset: Offset) -> Rect {
        Rect::new(
            self.left + offset.x,
            self.top + offset.y,
            self.width,
            self.height,
        )
    }
}

impl From<Size> for Rect {
    fn from(size: Size) -> Self {
        Self {
            left: as_const!(NonNaNFinite, f32, 0.0),
            top: as_const!(NonNaNFinite, f32, 0.0),
            width: size.width,
            height: size.height,
        }
    }
}

impl From<Rect> for accesskit::Rect {
    fn from(rect: Rect) -> Self {
        accesskit::Rect::new(
            f64::from(rect.left.get()),
            f64::from(rect.top.get()),
            f64::from(rect.right().get()),
            f64::from(rect.bottom().get()),
        )
    }
}
