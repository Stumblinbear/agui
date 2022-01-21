use std::ops::{Deref, DerefMut};

use agui_core::unit::{Point, Size};

#[derive(Debug, Default)]
pub struct WindowFocus(bool);

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

#[derive(Debug, Default)]
pub struct WindowPosition(Point);

impl Deref for WindowPosition {
    type Target = Point;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WindowPosition {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Default)]
pub struct WindowSize(Size);

impl Deref for WindowSize {
    type Target = Size;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WindowSize {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
