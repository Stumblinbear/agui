use glam::Mat4;

use crate::{offset::Offset, rect::Rect};

pub struct Canvas {}

impl Canvas {
    pub fn start_layer(&mut self, bounds: Rect) {
        let _ = bounds;
    }

    pub fn end_layer(&mut self) {}

    pub fn with_layer(&mut self, bounds: Rect, func: impl FnOnce(&mut Self)) {
        self.start_layer(bounds);
        func(self);
        self.end_layer();
    }

    pub fn start_transformation(&mut self, transformation: Mat4) {
        let _ = transformation;
    }

    pub fn end_transformation(&mut self) {}

    pub fn with_transformation(&mut self, transformation: Mat4, f: impl FnOnce(&mut Self)) {
        self.start_transformation(transformation);
        f(self);
        self.end_transformation();
    }

    pub fn with_offset(&mut self, offset: Offset, f: impl FnOnce(&mut Self)) {
        self.with_transformation(Mat4::from_translation(offset.into()), f);
    }
}
