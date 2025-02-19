use std::any::Any;

use glam::{Mat4, Vec3};

use crate::{offset::Offset, view_id::ViewPath};

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
    // pub element_id: ElementId,
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

    pub fn push_transform(&mut self, transform: Mat4) {
        self.transforms.push(self.current_transform() * transform);
    }

    pub fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    pub fn with_transform(
        &mut self,
        mut transform: Mat4,
        position: Offset,
        func: impl FnOnce(&mut Self, Offset) -> bool,
    ) -> bool {
        // Remove the perspective transform from the matrix
        transform.z_axis[0] = 0.0;
        transform.z_axis[1] = 0.0;
        transform.z_axis[2] = 1.0;
        transform.z_axis[3] = 0.0;

        transform.x_axis[2] = 0.0;
        transform.y_axis[2] = 0.0;
        transform.z_axis[2] = 1.0;
        transform.w_axis[2] = 0.0;

        if transform.determinant() == 0.0 {
            // Elements are not visible on screen and cannot be hit-tested.
            return false;
        }

        self.with_raw_transform(transform, position, func)
    }

    pub fn with_raw_transform(
        &mut self,
        transform: Mat4,
        position: Offset,
        func: impl FnOnce(&mut Self, Offset) -> bool,
    ) -> bool {
        // Transform the given position by the current transform
        let transformed_position = transform.transform_point3(position.into());

        self.transforms.push(self.current_transform() * transform);

        let result = func(
            self,
            Offset::new(transformed_position.x, transformed_position.y),
        );

        self.transforms.pop();

        result
    }

    pub fn with_offset(
        &mut self,
        offset: Offset,
        position: Offset,
        func: impl FnOnce(&mut Self, Offset) -> bool,
    ) -> bool {
        self.with_raw_transform(
            Mat4::from_translation(Vec3::new(-offset.x.get(), -offset.y.get(), 0.0)),
            position - offset,
            func,
        )
    }

    pub fn add(&mut self, view_path: ViewPath) {
        // self.path.push(HitTestEntry {
        //     element_id,
        //     data: None,
        //     transform: self.current_transform(),
        // });
    }

    //     pub fn add_with_data(&mut self, element_id: ElementId, data: impl Any) {
    //         self.path.push(HitTestEntry {
    //             element_id,
    //             data: Some(Box::new(data)),
    //             transform: self.current_transform(),
    //         });
    //     }
}
