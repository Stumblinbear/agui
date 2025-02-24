use std::any::Any;

use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::UpdateCtx,
    hit_test::HitTestResult,
    offset::Offset,
    render_object::{LayoutBoundMarker, LayoutIntrinsicMarker, RenderObject},
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
};

pub trait AnyRenderObject {
    type Width: LayoutBoundMarker;
    type Height: LayoutBoundMarker;

    type WidthIntrinsic: LayoutIntrinsicMarker;
    type HeightIntrinsic: LayoutIntrinsicMarker;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn render_object_name(&self) -> &str;

    fn dyn_mount(&mut self, ctx: &mut UpdateCtx);

    fn dyn_unmount(&mut self, ctx: &mut UpdateCtx);

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

    fn dyn_hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool;

    fn dyn_draw(&mut self, canvas: &mut Canvas);
}

impl<T, Width, Height, WidthIntrinsic, HeightIntrinsic> AnyRenderObject for T
where
    T: Any,
    T: RenderObject<
        Width = Width,
        Height = Height,
        WidthIntrinsic = WidthIntrinsic,
        HeightIntrinsic = HeightIntrinsic,
    >,
    Width: LayoutBoundMarker,
    Height: LayoutBoundMarker,
    WidthIntrinsic: LayoutIntrinsicMarker,
    HeightIntrinsic: LayoutIntrinsicMarker,
{
    type Width = Width;
    type Height = Height;

    type WidthIntrinsic = WidthIntrinsic;
    type HeightIntrinsic = HeightIntrinsic;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render_object_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_mount(&mut self, ctx: &mut UpdateCtx) {
        self.mount(ctx);
    }

    fn dyn_unmount(&mut self, ctx: &mut UpdateCtx) {
        self.unmount(ctx);
    }

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

    fn dyn_hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        self.hit_test(result, position)
    }

    fn dyn_draw(&mut self, canvas: &mut Canvas) {
        self.draw(canvas);
    }
}
impl<Width, Height, WidthIntrinsic, HeightIntrinsic> RenderObject
    for Box<
        dyn AnyRenderObject<
            Width = Width,
            Height = Height,
            WidthIntrinsic = WidthIntrinsic,
            HeightIntrinsic = HeightIntrinsic,
        >,
    >
where
    Width: LayoutBoundMarker,
    Height: LayoutBoundMarker,
    WidthIntrinsic: LayoutIntrinsicMarker,
    HeightIntrinsic: LayoutIntrinsicMarker,
{
    type Width = Width;
    type Height = Height;

    type WidthIntrinsic = WidthIntrinsic;
    type HeightIntrinsic = HeightIntrinsic;

    fn mount(&mut self, ctx: &mut UpdateCtx) {
        (**self).dyn_mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx) {
        (**self).dyn_unmount(ctx);
    }

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

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        (**self).dyn_hit_test(result, position)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        (**self).dyn_draw(canvas);
    }
}
