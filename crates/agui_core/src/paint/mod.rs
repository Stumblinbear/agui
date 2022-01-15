use slotmap::new_key_type;

pub mod canvas;

use self::canvas::Canvas;

new_key_type! {
    pub struct RenderId;
}

pub trait Painter {
    fn paint(&self, canvas: Canvas);
}
