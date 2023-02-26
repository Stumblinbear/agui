use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use agui_core::unit::Offset;

#[derive(Debug, Default, Clone, Copy)]
pub struct MousePos(pub Option<Offset>);

impl Deref for MousePos {
    type Target = Option<Offset>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MousePos {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Offset> for MousePos {
    fn from(point: Offset) -> Self {
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

#[derive(Debug, Default, Clone, Copy)]
pub struct Scroll(pub Offset);

impl Deref for Scroll {
    type Target = Offset;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Scroll {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Offset> for Scroll {
    fn from(point: Offset) -> Self {
        Scroll(point)
    }
}
