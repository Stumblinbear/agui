use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use agui_core::unit::Point;

#[derive(Debug, Default, Copy, Clone)]
pub struct MousePos(pub Option<Point>);

impl Deref for MousePos {
    type Target = Option<Point>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MousePos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Point> for MousePos {
    fn from(point: Point) -> Self {
        MousePos(Some(point))
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left(MouseButtonState),
    Right(MouseButtonState),
    Middle(MouseButtonState),
    Other(u16, MouseButtonState),
}

#[derive(Debug, Default, Clone)]
pub struct Mouse {
    pub pos: MousePos,
    pub button: MouseButtons,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct MouseButtons {
    pub left: MouseButtonState,
    pub right: MouseButtonState,
    pub middle: MouseButtonState,
    pub other: HashMap<u16, MouseButtonState>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseButtonState {
    Pressed,
    Released,
}

impl Default for MouseButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[derive(Debug, Default, Hash, Clone, Copy)]
pub struct Scroll(pub Point);

impl Deref for Scroll {
    type Target = Point;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Scroll {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Point> for Scroll {
    fn from(point: Point) -> Self {
        Scroll(point)
    }
}
