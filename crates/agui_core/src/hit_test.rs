use std::any::Any;

use glam::{Mat4, Vec3};

use crate::{offset::Offset, routing_id::RoutingPath};

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
        func: impl FnOnce(&mut Self, Offset) -> HitTest,
    ) -> HitTest {
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
            return HitTest::Pass;
        }

        self.with_raw_transform(transform, position, func)
    }

    pub fn with_raw_transform(
        &mut self,
        transform: Mat4,
        position: Offset,
        func: impl FnOnce(&mut Self, Offset) -> HitTest,
    ) -> HitTest {
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
        func: impl FnOnce(&mut Self, Offset) -> HitTest,
    ) -> HitTest {
        self.with_raw_transform(
            Mat4::from_translation(Vec3::new(-offset.x.get(), -offset.y.get(), 0.0)),
            position - offset,
            func,
        )
    }

    pub fn add(&mut self, path: RoutingPath) {
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

#[cfg(test)]
mod tests {
    use glam::Mat4;

    use crate::offset::Offset;

    use super::*;

    fn empty_result() -> HitTestResult {
        HitTestResult {
            path: Vec::new(),
            transforms: Vec::new(),
        }
    }

    #[test]
    fn default_transform_is_identity() {
        let result = empty_result();
        assert_eq!(result.current_transform(), Mat4::IDENTITY);
    }

    #[test]
    fn push_and_pop_transform() {
        let mut result = empty_result();
        let t = Mat4::from_translation(glam::Vec3::new(10.0, 20.0, 0.0));

        result.push_transform(t);
        assert_eq!(result.current_transform(), t);

        result.pop_transform();
        assert_eq!(result.current_transform(), Mat4::IDENTITY);
    }

    #[test]
    fn nested_transforms_compose() {
        let mut result = empty_result();
        let t1 = Mat4::from_translation(glam::Vec3::new(10.0, 0.0, 0.0));
        let t2 = Mat4::from_translation(glam::Vec3::new(0.0, 20.0, 0.0));

        result.push_transform(t1);
        result.push_transform(t2);

        let expected = t1 * t2;
        assert_eq!(result.current_transform(), expected);

        result.pop_transform();
        assert_eq!(result.current_transform(), t1);
    }

    #[test]
    fn with_offset_translates_position() {
        let mut result = empty_result();
        let offset = Offset::new(10.0_f32, 20.0_f32);
        let position = Offset::new(15.0_f32, 25.0_f32);

        let hit = result.with_offset(offset, position, |_, local_pos| {
            assert_eq!(local_pos.x.get(), 5.0);
            assert_eq!(local_pos.y.get(), 5.0);
            HitTest::Absorb
        });

        assert_eq!(hit, HitTest::Absorb);
        // Transform stack restored after callback
        assert_eq!(result.current_transform(), Mat4::IDENTITY);
    }

    #[test]
    fn with_transform_strips_perspective_and_bails_on_singular() {
        let mut result = empty_result();

        // A zero matrix has determinant 0 after perspective stripping
        let singular = Mat4::ZERO;
        let hit = result.with_transform(singular, Offset::ZERO, |_, _| {
            panic!("should not be called for singular matrix");
        });

        assert_eq!(hit, HitTest::Pass);
    }

    #[test]
    fn with_raw_transform_restores_stack() {
        let mut result = empty_result();
        let t = Mat4::from_scale(glam::Vec3::new(2.0, 2.0, 1.0));

        result.with_raw_transform(t, Offset::new(5.0_f32, 10.0_f32), |inner, pos| {
            // Position is transformed by the scale matrix
            assert_eq!(pos.x.get(), 10.0);
            assert_eq!(pos.y.get(), 20.0);
            // Inner transform stack should have the transform
            assert_eq!(inner.current_transform(), t);
            HitTest::Pass
        });

        assert_eq!(result.current_transform(), Mat4::IDENTITY);
    }
}
