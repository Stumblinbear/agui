use std::borrow::Cow;

use agui_core::{
    canvas::{paint::Brush, texture::TextureId},
    unit::{Bounds, FontStyle, Rect, Shape},
};

pub enum DrawCall {
    Layer {
        rect: Rect,
        shape: Shape,

        brush: Brush,
    },

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
