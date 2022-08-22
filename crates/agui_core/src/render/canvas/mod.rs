use crate::unit::{BlendMode, Rect, Shape};

pub mod command;
pub mod paint;
pub mod painter;

use self::command::CanvasCommand;

#[derive(Default, Debug)]
pub struct CanvasStyle {
    pub shape: Shape,

    pub anti_alias: bool,
    pub blend_mode: BlendMode,
}

#[derive(Default)]
pub struct Canvas {
    pub rect: Rect,

    pub head: Vec<CanvasCommand>,
    pub children: Vec<CanvasLayer>,
    pub tail: Option<Box<CanvasLayer>>,
}

pub struct CanvasLayer {
    pub style: CanvasStyle,

    pub canvas: Canvas,
}
