// use glyph_brush_layout::{
//     BuiltInLineBreaker, FontId, GlyphPositioner, SectionGeometry, SectionText, ToSectionText,
// };

// pub use glyph_brush_layout::ab_glyph::{Font, FontArc, ScaleFont};
// pub use glyph_brush_layout::Layout as GlyphLayout;
// pub use glyph_brush_layout::{HorizontalAlign, SectionGlyph, VerticalAlign};

use crate::unit::Color;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FontId(Option<usize>);

impl FontId {
    pub fn new(idx: usize) -> Self {
        Self(Some(idx))
    }

    pub fn idx(&self) -> Option<usize> {
        self.0
    }

    pub fn styled(&self) -> FontStyle {
        FontStyle {
            font_id: *self,
            color: Color::Black,
            ..FontStyle::default()
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FontStyle {
    pub font_id: FontId,

    pub size: f32,
    pub color: Color,

    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self {
            font_id: FontId(None),
            size: 32.0,
            color: Color::Black,

            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }
}

impl FontStyle {
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
