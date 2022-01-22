use crate::unit::{Rect, Shape};

use super::{clipping::Clip, font::FontStyle, paint::Brush};

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CanvasCommand {
    Clip {
        rect: Rect,
        clip: Clip,

        shape: Shape,
    },

    Shape {
        rect: Rect,
        brush: Brush,

        shape: Shape,
    },

    Text {
        rect: Rect,
        brush: Brush,

        font: FontStyle,
        text: String,
    },
}
