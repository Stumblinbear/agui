use glam::Mat4;

use crate::{offset::Offset, rect::Rect};

pub trait Renderer {
    fn start_layer(&mut self, bounds: Rect);

    fn end_layer(&mut self);

    fn with_layer(&mut self, bounds: Rect, func: impl FnOnce(&mut Self)) {
        self.start_layer(bounds);
        func(self);
        self.end_layer();
    }

    fn start_transformation(&mut self, transformation: Mat4);

    fn end_transformation(&mut self);

    fn with_transformation(&mut self, transformation: Mat4, f: impl FnOnce(&mut Self)) {
        self.start_transformation(transformation);
        f(self);
        self.end_transformation();
    }

    fn with_offset(&mut self, offset: Offset, f: impl FnOnce(&mut Self)) {
        self.with_transformation(Mat4::from_translation(offset.into()), f);
    }
}

impl Renderer for () {
    fn start_layer(&mut self, _: Rect) {}

    fn end_layer(&mut self) {}

    fn start_transformation(&mut self, _: Mat4) {}

    fn end_transformation(&mut self) {}
}
