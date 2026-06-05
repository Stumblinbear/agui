use typed_floats::{Positive, PositiveFinite, as_const};

use crate::{
    constraints::Constraints,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::PaintCtx,
    render_object::box_layout::RenderBox,
    size::Size,
    text_baseline::TextBaseline,
};

mod any_render_object;
pub mod box_layout;
mod layout;
mod node;
mod owner;
mod relayout_node;
pub mod sliver;

pub use any_render_object::*;
pub use layout::*;
pub use node::*;
pub use owner::*;
pub use relayout_node::*;

/// An object in the render tree.
///
/// A [`RenderObject`] has a lifecycle but does not itself define a coordinate system or layout
/// protocol. Those are introduced by the traits that extend it: [`RenderBox`], which lays out in
/// Cartesian coordinates, and [`RenderSliver`](crate::render_object::sliver::RenderSliver), which
/// lays out along a scroll axis.
pub trait RenderObject: 'static {
    fn mount(&mut self, ctx: &mut MountCtx);

    fn unmount(&mut self, ctx: &mut MountCtx);

    /// Recomputes whether this subtree contributes a compositing layer, and returns it. An
    /// implementation must fold in every child's bit; a node that reads its own bit while painting
    /// caches it here.
    fn update_compositing_bits(&mut self) -> bool;
}

#[derive(Default)]
pub struct RenderLeaf {}

impl RenderObject for RenderLeaf {
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, _: &mut MountCtx) {}

    fn update_compositing_bits(&mut self) -> bool {
        false
    }
}

impl RenderBox for RenderLeaf {
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

    fn layout(&mut self, _: &mut LayoutCtx, constraints: Constraints) -> Size {
        constraints.smallest()
    }

    fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        // TODO(trevin): should this add itself to the hit test result?
        HitTest::Pass
    }

    fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
}
