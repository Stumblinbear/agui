use std::borrow::Cow;

use crate::{
    render::texture::TextureId,
    unit::{Bounds, Color, FontStyle, Rect, Shape},
};

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum CanvasCommand {
    Shape {
        rect: Rect,
        shape: Shape,

        color: Color,
    },

    Texture {
        rect: Rect,
        shape: Shape,

        texture_id: TextureId,
        tex_bounds: Bounds,
    },

    Text {
        rect: Rect,

        color: Color,

        font: FontStyle,
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
