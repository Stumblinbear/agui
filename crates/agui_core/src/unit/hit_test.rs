use std::any::Any;

use glam::{Mat4, Vec3};

use crate::{render::RenderObjectId, unit::Offset};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitTest {
    /// The hit test was absorbed by the render object or at least one of its descendants.
    ///
    /// This prevents render objects below this one (i.e. its ancestors) from being hit.
    Absorb,

    /// The hit test was not absorbed by the render object.
    ///
    /// This allows render objects below this one (i.e. its ancestors) to be hit.
    Pass,
}

#[derive(Debug)]
pub struct HitTestEntry {
    pub render_object_id: RenderObjectId,
    pub data: Option<Box<dyn Any>>,
    transform: Mat4,
}

impl HitTestEntry {
    pub fn global_transform(&self) -> Mat4 {
        self.transform
    }
}

#[derive(Debug)]
pub struct HitTestResult {
    path: Vec<HitTestEntry>,
    transforms: Vec<Mat4>,
}

impl HitTestResult {
    fn current_transform(&self) -> Mat4 {
        self.transforms.last().copied().unwrap_or_default()
    }

    pub fn push_offset(&mut self, offset: Offset) {
        self.transforms.push(
            self.current_transform() * Mat4::from_translation(Vec3::new(offset.x, offset.y, 0.0)),
        );
    }

    pub fn push_transform(&mut self, transform: Mat4) {
        self.transforms.push(self.current_transform() * transform);
    }

    pub fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    pub fn add(&mut self, render_object_id: RenderObjectId) {
        self.path.push(HitTestEntry {
            render_object_id,
            data: None,
            transform: self.current_transform(),
        });
    }

    pub fn add_with_data(&mut self, render_object_id: RenderObjectId, data: impl Any) {
        self.path.push(HitTestEntry {
            render_object_id,
            data: Some(Box::new(data)),
            transform: self.current_transform(),
        });
    }
}
