// use glyph_brush_layout::{
//     BuiltInLineBreaker, FontId, GlyphPositioner, SectionGeometry, SectionText, ToSectionText,
// };

pub use glyph_brush_layout::ab_glyph::{Font, FontArc, ScaleFont};
pub use glyph_brush_layout::Layout as GlyphLayout;
pub use glyph_brush_layout::{HorizontalAlign, SectionGlyph, VerticalAlign};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct FontStyle {
    pub(crate) font_id: usize,
    pub(crate) font_size: f32,
}

impl Default for FontStyle {
    fn default() -> Self {
        Self {
            font_id: 0,
            font_size: 32.0,
        }
    }
}
