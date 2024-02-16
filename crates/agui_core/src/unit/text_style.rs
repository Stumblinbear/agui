use super::{Color, Font};

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font: Font,

    pub size: f32,
    pub color: Color,

    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
}

impl TextStyle {
    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn h_align(mut self, h_align: HorizontalAlign) -> Self {
        self.h_align = h_align;
        self
    }

    pub fn v_align(mut self, v_align: VerticalAlign) -> Self {
        self.v_align = v_align;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HorizontalAlign {
    Start,
    Center,
    End,
}

impl Default for HorizontalAlign {
    fn default() -> Self {
        Self::Start
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

impl Default for VerticalAlign {
    fn default() -> Self {
        Self::Top
    }
}
