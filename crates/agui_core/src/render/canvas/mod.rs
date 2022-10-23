use crate::unit::{BlendMode, Rect, Shape};

pub mod command;
pub mod painter;

pub use self::command::*;

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct Canvas {
    pub rect: Rect,

    pub head: Vec<CanvasCommand>,
    pub children: Vec<CanvasLayer>,
    pub tail: Option<Box<CanvasLayer>>,
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasLayer {
    pub style: LayerStyle,

    pub canvas: Canvas,
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct LayerStyle {
    pub shape: Shape,

    pub anti_alias: bool,
    pub blend_mode: BlendMode,
}
