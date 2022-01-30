use std::borrow::Cow;

use crate::unit::{Bounds, Rect, Shape, Size};

use super::{clipping::Clip, font::FontStyle, paint::Brush, texture::TextureId};

#[derive(Debug, Clone, Hash)]
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

    Texture {
        rect: Rect,
        brush: Brush,

        shape: Shape,

        texture: TextureId,
        tex_bounds: Bounds,
    },

    Text {
        rect: Rect,
        brush: Brush,

        font: FontStyle,

        text: Cow<'static, str>,
    },

    TextListener {
        size: Size,

        font: FontStyle,

        text: Cow<'static, str>,

        id: TextListenerId,
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
            CanvasCommand::Shape { brush, .. } | CanvasCommand::Text { brush, .. } => Some(*brush),
            _ => None,
        }
    }

    pub fn set_brush(&mut self, new_brush: Brush) {
        match self {
            CanvasCommand::Shape { brush, .. } | CanvasCommand::Text { brush, .. } => {
                *brush = new_brush;
            }
            _ => {}
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub struct TextListenerId(pub(crate) usize);
