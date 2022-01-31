use agui::canvas::{command::CanvasCommand, texture::TextureId};

use crate::render::context::RenderContext;

pub mod shape;
pub mod text;

use super::{BrushData, Layer};

#[derive(PartialEq)]
pub enum LayerType {
    Shape,
    Texture(TextureId),
    Text,
}

pub trait LayerBuilder<'builder> {
    fn get_type(&self) -> LayerType;

    fn can_process(&self, cmd: &CanvasCommand) -> bool;

    fn process(&mut self, cmd: CanvasCommand);

    fn build(&self, ctx: &mut RenderContext, brush_data: &[BrushData]) -> Option<Layer>;
}
