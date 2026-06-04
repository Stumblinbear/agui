use std::any::Any;

use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::PaintCtx,
    render_object::{AnyRenderObject, box_layout::RenderBox},
    size::Size,
    text_baseline::TextBaseline,
};

pub trait AnyRenderBox: AnyRenderObject {
    fn dyn_min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_measure(&self, constraints: Constraints) -> Size;

    fn dyn_layout(&mut self, constraints: Constraints) -> Size;

    fn dyn_measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>>;

    fn dyn_hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest;

    fn dyn_paint(&mut self, ctx: &mut PaintCtx, offset: Offset);
}

impl<T> AnyRenderBox for T
where
    T: Any,
    T: RenderBox,
{
    fn dyn_min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.min_intrinsic_width(height)
    }

    fn dyn_max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.max_intrinsic_width(height)
    }

    fn dyn_min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.min_intrinsic_height(width)
    }

    fn dyn_max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.max_intrinsic_height(width)
    }

    fn dyn_measure(&self, constraints: Constraints) -> Size {
        self.measure(constraints)
    }

    fn dyn_layout(&mut self, constraints: Constraints) -> Size {
        self.layout(constraints)
    }

    fn dyn_measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.measure_baseline(constraints, baseline)
    }

    fn dyn_distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.distance_to_baseline(baseline)
    }

    fn dyn_hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.hit_test(result, position)
    }

    fn dyn_paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.paint(ctx, offset);
    }
}

impl<T> RenderBox for Box<T>
where
    T: AnyRenderBox + ?Sized + 'static,
{
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        (**self).dyn_min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        (**self).dyn_max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        (**self).dyn_min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        (**self).dyn_max_intrinsic_height(width)
    }

    fn measure(&self, constraints: Constraints) -> Size {
        (**self).dyn_measure(constraints)
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        (**self).dyn_layout(constraints)
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        (**self).dyn_measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        (**self).dyn_distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        (**self).dyn_hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        (**self).dyn_paint(ctx, offset);
    }
}
