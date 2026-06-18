use std::{any::Any, cell::RefCell, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::PaintCtx,
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    render_object::{
        AnyRenderObject, LayoutCtx,
        box_layout::{BoxConstraints, RenderBox},
    },
    text::TextBaseline,
};

pub trait AnyRenderBox: AnyRenderObject {
    fn dyn_min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>>;

    fn dyn_measure(&self, constraints: BoxConstraints) -> Size;

    fn dyn_layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size;

    fn dyn_measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>>;

    fn dyn_hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest;

    fn dyn_update_compositing_bits(&mut self) -> bool;

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

    fn dyn_measure(&self, constraints: BoxConstraints) -> Size {
        self.measure(constraints)
    }

    fn dyn_layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout(ctx, constraints)
    }

    fn dyn_measure_baseline(
        &self,
        constraints: BoxConstraints,
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

    fn dyn_update_compositing_bits(&mut self) -> bool {
        self.update_compositing_bits()
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

    fn measure(&self, constraints: BoxConstraints) -> Size {
        (**self).dyn_measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        (**self).dyn_layout(ctx, constraints)
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
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

    fn update_compositing_bits(&mut self) -> bool {
        (**self).dyn_update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        (**self).dyn_paint(ctx, offset);
    }
}

impl<T> RenderBox for Rc<RefCell<T>>
where
    T: AnyRenderBox + ?Sized + 'static,
{
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().dyn_min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().dyn_max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().dyn_min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.borrow().dyn_max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.borrow().dyn_measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.borrow_mut().dyn_layout(ctx, constraints)
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.borrow().dyn_measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.borrow_mut().dyn_distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.borrow().dyn_hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.borrow_mut().dyn_update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.borrow_mut().dyn_paint(ctx, offset);
    }
}
