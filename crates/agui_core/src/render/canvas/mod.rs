use crate::unit::{BlendMode, Rect, Shape};

pub mod command;
pub mod paint;
pub mod painter;

use self::command::CanvasCommand;

#[derive(Default, Debug)]
pub struct CanvasStyle {
    pub rect: Rect,
    pub shape: Shape,

    pub anti_alias: bool,
    pub blend_mode: BlendMode,
}

#[derive(Default)]
pub struct Canvas {
    pub style: CanvasStyle,

    pub head: Vec<CanvasCommand>,
    pub children: Vec<Canvas>,
    pub tail: Vec<Canvas>,
}
