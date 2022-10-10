use crate::unit::{BlendMode, Rect, Shape};

pub mod command;
pub mod paint;
pub mod painter;

use self::command::CanvasCommand;

#[derive(Debug, Default, PartialEq)]
pub struct Canvas {
    pub rect: Rect,

    pub head: Vec<CanvasCommand>,
    pub children: Vec<CanvasLayer>,
    pub tail: Option<Box<CanvasLayer>>,
}

#[derive(Debug, PartialEq)]
pub struct CanvasLayer {
    pub style: LayerStyle,

    pub canvas: Canvas,
}

#[derive(Default, Debug, PartialEq)]
pub struct LayerStyle {
    pub shape: Shape,

    pub anti_alias: bool,
    pub blend_mode: BlendMode,
}
