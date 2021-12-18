#[derive(Default)]
pub struct MousePosition {
    pub x: f32,
    pub y: f32
}

pub struct MouseButton {
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
    pub right: MouseButtonState,
}

pub enum MouseButtonState {
    Up,
    Down,
    Held,
}

impl Default for MouseButtonState {
    fn default() -> Self {
        MouseButtonState::Up
    }
}