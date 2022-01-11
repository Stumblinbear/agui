#[derive(Debug, Default)]
pub struct Mouse {
    pub pos: Option<XY>,
    pub button: MouseButtons,
}

#[derive(Debug, Default)]
pub struct MouseButtons {
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
    pub right: MouseButtonState,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MouseButtonState {
    Pressed,
    Held,
    Released,
}

impl Default for MouseButtonState {
    fn default() -> Self {
        Self::Released
    }
}

#[derive(Debug, Default)]
pub struct Scroll {
    pub delta: XY,
}

#[derive(Debug, Default)]
pub struct XY {
    pub x: f64,
    pub y: f64,
}