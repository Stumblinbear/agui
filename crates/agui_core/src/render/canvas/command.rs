use std::borrow::Cow;

use crate::{
    render::texture::TextureId,
    unit::{Bounds, Rect, Shape, TextStyle},
};

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

        texture_id: TextureId,
        tex_bounds: Bounds,
    },

    Text {
        paint_idx: usize,

        rect: Rect,

        text_style: TextStyle,
        text: Cow<'static, str>,
    },
}

impl CanvasCommand {
    /// Returns `true` of the command will essentially do nothing.
    ///
    /// Generally this will be the case if the `rect` is a zero size, meaning nothing will be drawn.
    pub fn is_noop(&self) -> bool {
        match self {
            CanvasCommand::Shape { rect, .. }
            | CanvasCommand::Texture { rect, .. }
            | CanvasCommand::Text { rect, .. } => {
                rect.width.abs() <= f32::EPSILON || rect.height.abs() <= f32::EPSILON
            }
        }
    }
}
