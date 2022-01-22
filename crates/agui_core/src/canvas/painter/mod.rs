use super::Canvas;

pub mod shape;

pub trait CanvasPainter {
    fn draw(&self, canvas: &mut Canvas);
}