use std::any::Any;

use typed_floats::PositiveFinite;

use crate::{
    hit_test::{HitTest, HitTestResult},
    render_object::{
        AnyRenderObject,
        sliver::{RenderSliver, SliverConstraints, SliverGeometry},
    },
    renderer::Canvas,
};

pub trait AnyRenderSliver: AnyRenderObject {
    fn dyn_layout(&mut self, constraints: SliverConstraints) -> SliverGeometry;

    fn dyn_hit_test(
        &self,
        result: &mut HitTestResult,
        main_axis_position: PositiveFinite<f32>,
        cross_axis_position: PositiveFinite<f32>,
    ) -> HitTest;

    fn dyn_paint(&mut self, canvas: &mut Canvas);
}

impl<T> AnyRenderSliver for T
where
    T: Any,
    T: RenderSliver,
{
    fn dyn_layout(&mut self, constraints: SliverConstraints) -> SliverGeometry {
        RenderSliver::layout(self, constraints)
    }

    fn dyn_hit_test(
        &self,
        result: &mut HitTestResult,
        main_axis_position: PositiveFinite<f32>,
        cross_axis_position: PositiveFinite<f32>,
    ) -> HitTest {
        RenderSliver::hit_test(self, result, main_axis_position, cross_axis_position)
    }

    fn dyn_paint(&mut self, canvas: &mut Canvas) {
        RenderSliver::paint(self, canvas);
    }
}

impl<T> RenderSliver for Box<T>
where
    T: AnyRenderSliver + ?Sized + 'static,
{
    fn layout(&mut self, constraints: SliverConstraints) -> SliverGeometry {
        (**self).dyn_layout(constraints)
    }

    fn hit_test(
        &self,
        result: &mut HitTestResult,
        main_axis_position: PositiveFinite<f32>,
        cross_axis_position: PositiveFinite<f32>,
    ) -> HitTest {
        (**self).dyn_hit_test(result, main_axis_position, cross_axis_position)
    }

    fn paint(&mut self, canvas: &mut Canvas) {
        (**self).dyn_paint(canvas);
    }
}
