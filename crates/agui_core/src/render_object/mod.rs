use typed_floats::{Positive, PositiveFinite, as_const};

use crate::{
    constraints::Constraints,
    context::UpdateCtx,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::box_layout::BoxLayout,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
};

mod any_render_object;
pub mod box_layout;
mod node;

pub use any_render_object::*;
pub use node::*;

pub trait RenderObject: 'static {
    fn mount(&mut self, ctx: &mut UpdateCtx);

    fn unmount(&mut self, ctx: &mut UpdateCtx);

    /// Determines the set of [`View`]s located at the given position.
    ///
    /// Returns [`HitTest::Absorb`], and adds any [`View`]s that contain the
    /// point to the given hit test result, if this [`View`] or one of its
    /// descendants absorbs the hit (preventing [`View`]s below this one from
    /// being hit). Returns [`HitTest::Pass`] if the hit can continue to other
    /// [`View`]s below this one.
    ///
    /// The caller is responsible for transforming `position` from global
    /// coordinates to its location relative to the origin of this [`View`].
    /// This [`View`] is responsible for checking whether the given position is
    /// within its bounds.s
    ///
    /// Hit testing requires layout to be up-to-date but does not require drawing
    /// to be up-to-date. That means an [`View`] can rely upon [`View::layout`]
    /// having been called in [`View::hit_test`] but cannot rely upon [`View::draw`]
    /// having been called.
    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest;

    fn paint(&mut self, canvas: &mut Canvas);
}

#[derive(Default)]
pub struct RenderLeaf {}

impl RenderObject for RenderLeaf {
    fn mount(&mut self, _: &mut UpdateCtx) {}

    fn unmount(&mut self, _: &mut UpdateCtx) {}

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        // TODO(trevin): should this add itself to the hit test result?
        HitTest::Pass
    }

    fn paint(&mut self, _: &mut Canvas) {}
}

impl BoxLayout for RenderLeaf {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        Some(as_const!(PositiveFinite, f32, 0.0))
    }

    fn measure(&self, constraints: Constraints) -> Size {
        constraints.smallest()
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        constraints.smallest()
    }

    fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }
}
