use super::Canvas;

pub mod shape;

pub trait Painter {
    fn paint(&self, canvas: &mut Canvas);
}