use agui::canvas::command::CanvasCommand;

use crate::render::context::RenderContext;

pub mod shape;
pub mod text;

use super::{BrushData, DrawCall};

pub trait DrawCallBuilder<'builder> {
    fn can_process(&self, cmd: &CanvasCommand) -> bool;

    fn process(&mut self, cmd: CanvasCommand);

    fn build(&self, ctx: &mut RenderContext, brush_data: &[BrushData]) -> Option<DrawCall>;
}
