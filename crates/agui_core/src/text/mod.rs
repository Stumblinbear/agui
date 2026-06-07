mod fonts;
mod paragraph;
mod text_baseline;
mod text_direction;

use peniko::{Brush, Color};

pub use fonts::Fonts;
pub use paragraph::*;
pub use text_baseline::TextBaseline;
pub use text_direction::TextDirection;

/// The brush a shaped glyph run is painted with.
#[derive(Clone, PartialEq, Debug)]
pub struct TextBrush(pub Brush);

impl Default for TextBrush {
    fn default() -> Self {
        Self(Brush::Solid(Color::BLACK))
    }
}

impl From<Brush> for TextBrush {
    fn from(brush: Brush) -> Self {
        Self(brush)
    }
}

impl From<Color> for TextBrush {
    fn from(color: Color) -> Self {
        Self(Brush::Solid(color))
    }
}
