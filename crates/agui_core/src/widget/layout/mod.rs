use crate::{
    gestures::hit_test::HitTestEntry,
    unit::{Constraints, IntrinsicDimension, Offset, Size},
};

mod context;
mod instance;
mod iter;

pub use context::*;
pub use instance::*;
pub use iter::*;

use super::{BuildContext, Widget};

pub trait WidgetLayout: Sized + 'static {
    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> Vec<Widget>;

    #[allow(unused_variables)]
    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        // By default we defer to the first child

        ctx.iter_children().next().map_or(0.0, |child| {
            child.compute_intrinsic_size(dimension, cross_extent)
        })
    }

    #[allow(unused_variables)]
    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size;

    /// Checks if the given position "hits" this widget or any of its children.
    ///
    /// Returns `true` if the given point is contained within this widget or one of its
    /// descendants. One should make sure to add themselves to the result if its children
    /// are hit.
    fn hit_test(&self, ctx: &mut HitTestContext, position: Offset) -> bool {
        let mut hit = false;

        if ctx.size.contains(position) {
            while let Some(mut child) = ctx.iter_children().next_back() {
                if child.hit_test(position) {
                    hit = true;

                    break;
                }
            }
        }

        if hit {
            ctx.add_result(HitTestEntry {
                element_id: ctx.element_id,
                position,
            });
        }

        hit
    }
}
