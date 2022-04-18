use std::borrow::Cow;

use crate::unit::{Bounds, FontStyle, Rect, Shape};

use super::{paint::Brush, texture::TextureId};

#[derive(Debug, Clone, Hash)]
#[non_exhaustive]
pub enum CanvasCommand {
    Layer {
        rect: Rect,
        shape: Shape,

        brush: Brush,
    },

    Pop,

    Shape {
        rect: Rect,
        shape: Shape,

        brush: Brush,
    },

    Texture {
        rect: Rect,
        shape: Shape,

        brush: Brush,

        texture_id: TextureId,
        tex_bounds: Bounds,
    },

    Text {
        rect: Rect,

        brush: Brush,

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

    pub fn get_brush(&self) -> Option<Brush> {
        match self {
            CanvasCommand::Layer { brush, .. }
            | CanvasCommand::Shape { brush, .. }
            | CanvasCommand::Texture { brush, .. }
            | CanvasCommand::Text { brush, .. } => Some(*brush),
            _ => None,
        }
    }

    pub fn set_brush(&mut self, new_brush: Brush) {
        match self {
            CanvasCommand::Layer { brush, .. }
            | CanvasCommand::Shape { brush, .. }
            | CanvasCommand::Texture { brush, .. }
            | CanvasCommand::Text { brush, .. } => {
                *brush = new_brush;
            }
            _ => {}
        }
    }
}
