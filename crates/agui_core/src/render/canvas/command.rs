use std::borrow::Cow;

use crate::unit::{Bounds, Rect, Shape, TextStyle, Texture};

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum CanvasCommand {
    Shape {
        paint_idx: usize,

        rect: Rect,
        shape: Shape,
    },

    Texture {
        rect: Rect,
        shape: Shape,

        texture: Texture,
        tex_bounds: Bounds,
    },

    Text {
        paint_idx: usize,

        rect: Rect,

        text_style: TextStyle,
        text: Cow<'static, str>,
    },
}
