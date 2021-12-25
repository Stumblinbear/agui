#[derive(Default)]
pub struct WindowFocus(pub bool);

#[derive(Default)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Default)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}