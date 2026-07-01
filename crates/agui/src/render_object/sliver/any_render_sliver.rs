use std::{any::Any, cell::RefCell, rc::Rc};

use typed_floats::PositiveFinite;

use crate::{
    context::PaintCtx,
    geometry::Offset,
    input::hit_test::{HitTest, HitTestResult},
    render_object::{
        AnyRenderObject,
        sliver::{RenderSliver, SliverConstraints, SliverGeometry},
    },
    semantics::SemanticsTreeBuilder,
};

pub trait AnyRenderSliver: AnyRenderObject {
    fn dyn_layout(&mut self, constraints: SliverConstraints) -> SliverGeometry;

    fn dyn_hit_test(
        &self,
        result: &mut HitTestResult,
        main_axis_position: PositiveFinite<f32>,
        cross_axis_position: PositiveFinite<f32>,
    ) -> HitTest;

    fn dyn_paint(&mut self, ctx: &mut PaintCtx, offset: Offset);

    fn dyn_build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>);
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

    fn dyn_paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        RenderSliver::paint(self, ctx, offset);
    }

    fn dyn_build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.build_semantics(s);
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

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        (**self).dyn_paint(ctx, offset);
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        (**self).dyn_build_semantics(s);
    }
}

impl<T> RenderSliver for Rc<RefCell<T>>
where
    T: AnyRenderSliver + ?Sized + 'static,
{
    fn layout(&mut self, constraints: SliverConstraints) -> SliverGeometry {
        self.borrow_mut().dyn_layout(constraints)
    }

    fn hit_test(
        &self,
        result: &mut HitTestResult,
        main_axis_position: PositiveFinite<f32>,
        cross_axis_position: PositiveFinite<f32>,
    ) -> HitTest {
        self.borrow()
            .dyn_hit_test(result, main_axis_position, cross_axis_position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.borrow_mut().dyn_paint(ctx, offset);
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.borrow_mut().dyn_build_semantics(s);
    }
}
