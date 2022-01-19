use std::fs::File;
use std::io::{self, BufReader, Read};

use agui_core::canvas::font::FontDescriptor;

pub use glyph_brush_layout::ab_glyph::{Font, FontArc, ScaleFont};
pub use glyph_brush_layout::Layout as GlyphLayout;

#[derive(Debug, Default)]
pub struct Fonts {
    fonts: Vec<FontArc>,
}

impl Fonts {
    pub fn get_fonts(&self) -> &Vec<FontArc> {
        &self.fonts
    }

    pub fn get(&self, font_id: FontDescriptor) -> FontArc {
        FontArc::clone(&self.fonts[font_id.0])
    }

    pub fn load_bytes(&mut self, bytes: &'static [u8]) -> (FontDescriptor, FontArc) {
        let font = FontArc::try_from_slice(bytes).unwrap();

        self.fonts.push(FontArc::clone(&font));

        (FontDescriptor(self.fonts.len() - 1), font)
    }

    pub fn load_file(&mut self, filename: &str) -> io::Result<(FontDescriptor, FontArc)> {
        let f = File::open(filename)?;
        let mut reader = BufReader::new(f);

        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes)?;

        let font = FontArc::try_from_vec(bytes).unwrap();

        self.fonts.push(FontArc::clone(&font));

        Ok((FontDescriptor(self.fonts.len() - 1), font))
    }
}
