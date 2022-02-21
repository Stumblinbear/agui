use agui_core::unit::Point;

#[derive(Debug, Default, Clone)]
pub struct Mouse {
    pub pos: Option<Point>,
    pub button: MouseButtons,
}

#[derive(Debug, Default, Clone)]
pub struct MouseButtons {
    pub left: MouseButtonState,
    pub middle: MouseButtonState,
    pub right: MouseButtonState,
}

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, Default, Clone)]
pub struct Scroll {
    pub delta: Point,
}
