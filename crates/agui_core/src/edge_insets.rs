use typed_floats::{PositiveFinite, as_const};

use crate::{axis::Axis, text_direction::TextDirection};

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

    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top: PositiveFinite::try_from(top).expect("top must be a positive finite number"),
            right: PositiveFinite::try_from(right).expect("right must a positive finite number"),
            bottom: PositiveFinite::try_from(bottom).expect("bottom must a positive finite number"),
            left: PositiveFinite::try_from(left).expect("left must a positive finite number"),
        }
    }

    pub fn all(value: f32) -> Self {
        let value =
            PositiveFinite::try_from(value).expect("value must be a positive finite number");

        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
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

    pub fn new(start: f32, top: f32, end: f32, bottom: f32) -> Self {
        Self {
            start: PositiveFinite::try_from(start).expect("start must be a positive finite number"),
            top: PositiveFinite::try_from(top).expect("top must a positive finite number"),
            end: PositiveFinite::try_from(end).expect("end must a positive finite number"),
            bottom: PositiveFinite::try_from(bottom).expect("bottom must a positive finite number"),
        }
    }

    pub fn horizontal(&self) -> PositiveFinite<f32> {
        PositiveFinite::try_from(self.start + self.end)
            .expect("total horizontal edge insets must be a positive finite number")
    }

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
