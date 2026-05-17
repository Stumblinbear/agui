use std::any::Any;

use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    render_object::{
        box_layout::{BoxLayout, RenderBox},
        AnyRenderObject,
    },
    size::Size,
    text_baseline::TextBaseline,
};

pub trait AnyRenderBox: AnyRenderObject {
    fn dyn_size(&self) -> Size;

    fn dyn_min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_measure(&self, constraints: Constraints) -> Size;

    fn dyn_layout(&mut self, constraints: Constraints);

    fn dyn_measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>>;
}

impl<T> AnyRenderBox for T
where
    T: Any,
    T: RenderBox,
{
    fn dyn_size(&self) -> Size {
        self.size()
    }

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

    fn dyn_layout(&mut self, constraints: Constraints) {
        self.layout(constraints);
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
}

impl<T> BoxLayout for Box<T>
where
    T: AnyRenderBox + ?Sized + 'static,
{
    fn size(&self) -> Size {
        (**self).dyn_size()
    }

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

    fn layout(&mut self, constraints: Constraints) {
        (**self).dyn_layout(constraints);
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
}
