use std::fs::File;
use std::io::{self, BufReader, Read};

pub use glyph_brush_layout::ab_glyph::FontArc;
pub use glyph_brush_layout::FontId;
pub use glyph_brush_layout::Layout as GlyphLayout;

#[derive(Default)]
pub struct Fonts {
    fonts: Vec<FontArc>,
}

impl Fonts {
    pub fn get_fonts(&self) -> &Vec<FontArc> {
        &self.fonts
    }

    pub fn load_bytes(&mut self, bytes: &'static [u8]) -> (FontId, FontArc) {
        let font = FontArc::try_from_slice(bytes).unwrap();

        self.fonts.push(FontArc::clone(&font));

        (FontId(self.fonts.len() - 1), font)
    }

    pub fn load_file(&mut self, filename: &str) -> io::Result<(FontId, FontArc)> {
        let f = File::open(filename)?;
        let mut reader = BufReader::new(f);

        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes)?;

        let font = FontArc::try_from_vec(bytes).unwrap();

        self.fonts.push(FontArc::clone(&font));

        Ok((FontId(self.fonts.len() - 1), font))
    }
}
