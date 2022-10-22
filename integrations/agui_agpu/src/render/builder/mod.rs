use agui::render::canvas::command::CanvasCommand;

use crate::context::RenderContext;

use super::draw_call::DrawCall;

pub mod shape;
pub mod text;

pub trait DrawCallBuilder<'builder> {
    fn can_process(&self, cmd: &CanvasCommand) -> bool;

    fn process(&mut self, cmd: CanvasCommand);

    fn build(&self, ctx: &mut RenderContext) -> Option<DrawCall>;
}
