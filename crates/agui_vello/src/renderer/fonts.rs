use std::sync::Arc;

use agui_core::unit::{Font, FontData};
use rustc_hash::FxHashMap;
use vello::{
    glyph::{
        skrifa::{setting::Setting, FontRef},
        GlyphContext, GlyphProvider,
    },
    peniko::{self, Blob},
};

pub struct VelloFonts {
    glyph_context: GlyphContext,
    fonts: FxHashMap<Font, peniko::Font>,
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
        ppem: f32,
        hint: bool,
        variations: V,
    ) -> GlyphProvider<'a>
    where
        V: IntoIterator,
        V::Item: Into<Setting<f32>>,
    {
        self.glyph_context
            .new_provider(font, ppem, hint, variations)
    }

    pub fn get(&self, font: &Font) -> Option<&peniko::Font> {
        self.fonts.get(font)
    }

    pub fn get_or_insert(&mut self, font: Font) -> &peniko::Font {
        self.fonts
            .entry(font)
            .or_insert_with_key(|font| match font.as_ref() {
                FontData::Bytes(bytes) => {
                    peniko::Font::new(Blob::new(Arc::clone(bytes) as Arc<_>), 0)
                }

                _ => todo!("font data not supported"),
            })
    }

    pub fn to_font_ref(font: &peniko::Font) -> Option<FontRef<'_>> {
        use vello::skrifa::raw::FileRef;

        let file_ref = FileRef::new(font.data.as_ref()).ok()?;

        match file_ref {
            FileRef::Font(font) => Some(font),
            FileRef::Collection(collection) => collection.get(font.index).ok(),
        }
    }
}
