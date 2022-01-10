use std::fs::File;
use std::io::{self, BufReader, Read};

use glyph_brush_layout::FontId;

pub use glyph_brush_layout::ab_glyph::{Font, FontArc};
pub use glyph_brush_layout::Layout as GlyphLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct FontDescriptor(pub usize);

impl From<FontDescriptor> for FontId {
    fn from(font: FontDescriptor) -> Self {
        FontId(font.0)
    }
}

#[derive(Default)]
pub struct Fonts {
    fonts: Vec<FontArc>,
}

impl Fonts {
    pub fn get_fonts(&self) -> &Vec<FontArc> {
        &self.fonts
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
