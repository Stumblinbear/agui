use std::ops::{Deref, DerefMut};

use agui_core::unit::{Point, Size};

#[derive(Debug, Default, Hash, PartialEq, Eq, Clone, Copy)]
pub struct WindowFocus(pub bool);

impl Deref for WindowFocus {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WindowFocus {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct WindowPosition {
    pub x: f32,
    pub y: f32,
}

impl From<Point> for WindowPosition {
    fn from(point: Point) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
pub struct WindowSize {
    pub width: f32,
    pub height: f32,
}

impl From<Size> for WindowSize {
    fn from(size: Size) -> Self {
        Self {
            width: size.width,
            height: size.height,
        }
    }
}
