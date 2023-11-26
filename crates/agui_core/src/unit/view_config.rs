use super::Size;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ViewConfiguration {
    pub size: Size,

    pub device_pixel_ratio: f32,
}
