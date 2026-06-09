mod fonts;
mod inline_span;
mod paragraph;
mod text_baseline;
mod text_direction;
mod text_style;

use peniko::{Brush, Color};

pub use fonts::Fonts;
pub use inline_span::*;
pub use paragraph::*;
pub use text_baseline::TextBaseline;
pub use text_direction::TextDirection;
pub use text_style::TextStyle;

pub use parley::style::{FontStyle, FontWeight, FontWidth, LineHeight};

/// How a span of text is painted: the [`fill`](Self::fill) of its glyphs and an optional
/// [`background`](Self::background) drawn behind them.
///
/// Each shaped run carries its own brush, so one paragraph paints in several colors and highlights.
#[derive(Clone, PartialEq, Debug)]
pub struct TextBrush {
    /// The fill applied to the glyphs themselves.
    pub fill: Brush,
    /// A fill drawn as a solid block behind the run, or `None` for no highlight.
    pub background: Option<Brush>,
}

impl Default for TextBrush {
    fn default() -> Self {
        Self {
            fill: Brush::Solid(Color::BLACK),
            background: None,
        }
    }
}

impl From<Brush> for TextBrush {
    fn from(fill: Brush) -> Self {
        Self {
            fill,
            background: None,
        }
    }
}

impl From<Color> for TextBrush {
    fn from(color: Color) -> Self {
        Self {
            fill: Brush::Solid(color),
            background: None,
        }
    }
}
