use agui_core::unit::Font;
use rustc_hash::FxHashMap;
use vello::{
    fello::{raw::FontRef, FontKey, Setting},
    glyph::{GlyphContext, GlyphProvider},
};

pub struct VelloFonts {
    glyph_context: GlyphContext,
    fonts: FxHashMap<Font, FontRef<'static>>,
}

impl Default for VelloFonts {
    fn default() -> Self {
        Self {
            glyph_context: GlyphContext::new(),
            fonts: FxHashMap::default(),
        }
    }
}

impl VelloFonts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_provider<'a, V>(
        &'a mut self,
        font: &FontRef<'a>,
        font_id: Option<FontKey>,
        ppem: f32,
        hint: bool,
        variations: V,
    ) -> GlyphProvider<'a>
    where
        V: IntoIterator,
        V::Item: Into<Setting<f32>>,
    {
        self.glyph_context
            .new_provider(font, font_id, ppem, hint, variations)
    }

    // pub fn add_font(&mut self, font: FontRef<'static>) -> Font {
    //     let font_id = Font::by_id(self.fonts.len());

    //     self.fonts.insert(font_id, font);

    //     if self.default_font.is_none() {
    //         self.default_font = Some(font_id);
    //     }

    //     font_id
    // }

    pub fn get(&self, font: Font) -> Option<FontRef<'static>> {
        self.fonts.get(&font).cloned()
    }
}
