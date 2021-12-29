#[derive(Default)]
pub struct Mouse {
    pub pos: Option<XY>,
    pub button: MouseButtons,
}

#[derive(Default)]
pub struct MouseButtons {
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
    pub right: MouseButtonState,
}

#[derive(PartialEq, Eq)]
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
    pub delta: XY,
}

#[derive(Default)]
pub struct XY {
    pub x: f64,
    pub y: f64,
}