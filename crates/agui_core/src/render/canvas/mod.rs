use crate::unit::{Offset, Shape, Size};

pub mod command;
pub mod painter;

pub use self::command::*;

use super::Paint;

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct Canvas {
    pub size: Size,

    pub paints: Vec<Paint>,

    pub head: Vec<CanvasCommand>,
    pub children: Vec<CanvasLayer>,
    pub tail: Option<Box<CanvasLayer>>,
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasLayer {
    pub offset: Offset,

    pub style: LayerStyle,

    pub canvas: Canvas,
}

#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct LayerStyle {
    pub paint_idx: usize,

    pub shape: Shape,
}
