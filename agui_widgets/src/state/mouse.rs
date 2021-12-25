#[derive(Default)]
pub struct Mouse {
    pub pos: Option<MousePosition>,
    pub button: MouseButtons,
}

#[derive(Default)]
pub struct MousePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Default)]
pub struct MouseButtons {
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
    pub right: MouseButtonState,
}

pub enum MouseButtonState {
    Pressed,
    Released,
}

impl Default for MouseButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[derive(Default)]
pub struct Scroll {
    pub delta: ScrollDelta,
}

#[derive(Default)]
pub struct ScrollDelta {
    pub x: f64,
    pub y: f64,
}
