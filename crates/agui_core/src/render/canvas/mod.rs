use crate::unit::{BlendMode, Point, Shape, Size};

pub mod command;
pub mod painter;

pub use self::command::*;

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct Canvas {
    pub size: Size,

    pub head: Vec<CanvasCommand>,
    pub children: Vec<CanvasLayer>,
    pub tail: Option<Box<CanvasLayer>>,
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasLayer {
    pub offset: Point,

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
