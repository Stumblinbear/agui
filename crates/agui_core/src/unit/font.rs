use std::{ops::Deref, sync::Arc};

use glyph_brush_layout::{
    ab_glyph::{Font as GlyphFont, FontArc, GlyphId, InvalidFont, ScaleFont},
    BuiltInLineBreaker, FontId as GlyphFontId, GlyphPositioner, Layout as GlyphLayout,
    SectionGeometry, SectionGlyph, SectionText,
};

use crate::unit::{Color, Rect};

// TODO: fonts should use a `usize` so that we aren't locked into a single text handling library
#[derive(Debug, Clone, Default)]
pub struct Font(Option<FontArc>);

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            // war crimes
            (Some(self_font), Some(other_font)) => std::ptr::eq(
                Arc::as_ptr(&Arc::from(self_font)) as *const _ as *const (),
                Arc::as_ptr(&Arc::from(other_font)) as *const _ as *const (),
            ),
            (Some(_), None) | (None, Some(_)) => false,
            (None, None) => true,
        }
    }
}

impl Font {
    pub fn styled(&self) -> FontStyle {
        FontStyle {
            font: self.clone(),
            color: Color {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            },

            ..FontStyle::default()
        }
    }
}

impl Font {
    pub fn try_from_slice(bytes: &'static [u8]) -> Result<Self, InvalidFont> {
        FontArc::try_from_slice(bytes).map(Self::from)
    }

    pub fn try_from_vec(bytes: Vec<u8>) -> Result<Self, InvalidFont> {
        FontArc::try_from_vec(bytes).map(Self::from)
    }
}

impl From<FontArc> for Font {
    fn from(font: FontArc) -> Self {
        Self(Some(font))
    }
}

impl Deref for Font {
    type Target = Option<FontArc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontStyle {
    pub font: Font,

    pub size: f32,
    pub color: Color,

    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self {
            font: Font(None),
            size: 16.0,
            color: Color {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            },

            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        }
    }
}

impl FontStyle {
    pub fn get_font(&self) -> Option<&FontArc> {
        self.font.as_ref()
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

    pub fn h_advance(&self, glyph_id: GlyphId) -> f32 {
        self.get_font()
            .map(|font| font.as_scaled(self.size).h_advance(glyph_id))
            .unwrap_or(0.0)
    }

    pub fn v_advance(&self, glyph_id: GlyphId) -> f32 {
        self.get_font()
            .map(|font| font.as_scaled(self.size).v_advance(glyph_id))
            .unwrap_or(0.0)
    }

    pub fn get_glyphs(&self, mut rect: Rect, text: &str) -> Vec<SectionGlyph> {
        if text.is_empty() {
            return Vec::new();
        }

        self.get_font().map_or_else(Vec::default, |font| {
            let glyphs_layout = GlyphLayout::Wrap {
                line_breaker: BuiltInLineBreaker::UnicodeLineBreaker,
                h_align: match self.h_align {
                    HorizontalAlign::Left => glyph_brush_layout::HorizontalAlign::Left,
                    HorizontalAlign::Center => {
                        rect.left += rect.width / 2.0;

                        glyph_brush_layout::HorizontalAlign::Center
                    }

                    HorizontalAlign::Right => {
                        rect.left += rect.width;

                        glyph_brush_layout::HorizontalAlign::Right
                    }
                },
                v_align: match self.v_align {
                    VerticalAlign::Top => glyph_brush_layout::VerticalAlign::Top,
                    VerticalAlign::Center => {
                        rect.top += rect.height / 2.0;

                        glyph_brush_layout::VerticalAlign::Center
                    }

                    VerticalAlign::Bottom => {
                        rect.top += rect.height;

                        glyph_brush_layout::VerticalAlign::Bottom
                    }
                },
            };

            glyphs_layout.calculate_glyphs(
                &[font],
                &SectionGeometry {
                    screen_position: (rect.left, rect.top),
                    bounds: (rect.width, rect.height),
                },
                &[SectionText {
                    text,
                    scale: self.size.into(),
                    font_id: GlyphFontId(0),
                }],
            )
        })
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
