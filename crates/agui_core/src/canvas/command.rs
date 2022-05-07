use std::borrow::Cow;

use crate::unit::{BlendMode, Bounds, Color, FontStyle, Rect, Shape};

use super::texture::TextureId;

#[derive(Debug, Clone, Hash)]
#[non_exhaustive]
pub enum CanvasCommand {
    Layer {
        rect: Rect,
        shape: Shape,

        anti_alias: bool,
        blend_mode: BlendMode,
    },

    Pop,

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
            CanvasCommand::Shape { rect, .. } | CanvasCommand::Text { rect, .. } => {
                if rect.width.abs() <= f32::EPSILON || rect.height.abs() <= f32::EPSILON {
                    return true;
                }
            }
            _ => {}
        }

        false
    }
}
