#[derive(Debug, Default)]
pub struct WindowFocus(pub bool);

#[derive(Debug, Default)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Default)]
pub struct WindowSize {
    pub width: u32,
    pub height: u32,
}