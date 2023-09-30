use crate::unit::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Font(usize);

impl Font {
    pub fn by_id(font_id: usize) -> Self {
        Self(font_id)
    }

    pub fn styled(&self) -> TextStyle {
        TextStyle {
            font: Some(*self),

            ..TextStyle::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font: Option<Font>,

    pub size: f32,
    pub color: Color,

    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: None,

            size: 16.0,
            color: Color {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            },

            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
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
    Left,
    Center,
    Right,
}

impl Default for HorizontalAlign {
    fn default() -> Self {
        Self::Left
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
