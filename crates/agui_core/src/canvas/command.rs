use crate::unit::{Bounds, Color, Shape};

use super::font::FontDescriptor;

#[derive(PartialEq)]
pub enum CanvasCommand {
    Shape {
        bounds: Bounds,

        shape: Shape,

        color: Color,
    },

    Text {
        bounds: Bounds,

        font: FontDescriptor,
        text: String,

        color: Color,
    },
}
