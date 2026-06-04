use peniko::kurbo::{self, Shape};

use crate::{offset::Offset, rect::Rect, size::Size};

/// Maximum error, in paint-space units, allowed when approximating a curved shape as a path.
pub(crate) const DEFAULT_TOLERANCE: f64 = 0.1;

impl From<Offset> for kurbo::Vec2 {
    fn from(offset: Offset) -> Self {
        kurbo::Vec2::new(f64::from(offset.x.get()), f64::from(offset.y.get()))
    }
}

impl From<Offset> for kurbo::Point {
    fn from(offset: Offset) -> Self {
        kurbo::Point::new(f64::from(offset.x.get()), f64::from(offset.y.get()))
    }
}

impl From<kurbo::Point> for Offset {
    #[allow(clippy::cast_possible_truncation)]
    fn from(point: kurbo::Point) -> Self {
        Offset::new(point.x as f32, point.y as f32)
    }
}

impl From<Size> for kurbo::Size {
    fn from(size: Size) -> Self {
        kurbo::Size::new(f64::from(size.width.get()), f64::from(size.height.get()))
    }
}

impl From<Rect> for kurbo::Rect {
    fn from(rect: Rect) -> Self {
        let left = f64::from(rect.left.get());
        let top = f64::from(rect.top.get());

        kurbo::Rect::new(
            left,
            top,
            left + f64::from(rect.width.get()),
            top + f64::from(rect.height.get()),
        )
    }
}

impl Shape for Rect {
    type PathElementsIter<'iter> = kurbo::RectPathIter;

    fn path_elements(&self, tolerance: f64) -> Self::PathElementsIter<'_> {
        kurbo::Rect::from(*self).path_elements(tolerance)
    }

    fn as_rect(&self) -> Option<kurbo::Rect> {
        Some(kurbo::Rect::from(*self))
    }

    fn area(&self) -> f64 {
        kurbo::Rect::from(*self).area()
    }

    fn perimeter(&self, accuracy: f64) -> f64 {
        kurbo::Rect::from(*self).perimeter(accuracy)
    }

    fn winding(&self, pt: kurbo::Point) -> i32 {
        kurbo::Rect::from(*self).winding(pt)
    }

    fn bounding_box(&self) -> kurbo::Rect {
        kurbo::Rect::from(*self)
    }
}
