use crate::unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size};

mod context;
mod instance;
mod iter;

pub use context::*;
pub use instance::*;
pub use iter::*;

use super::Widget;

pub trait WidgetLayout: Sized + 'static {
    fn get_children(&self) -> Vec<Widget>;

    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32;

    #[allow(unused_variables)]
    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size;

    /// Checks if the given position "hits" this widget or any of its descendants.
    ///
    /// Returns `true` if the given point is contained within this widget or one of its
    /// descendants. One should make sure to add themselves to the result if its children
    /// are hit.
    ///
    /// The given position should be in the widget's local coordinate space, not the global
    /// coordinate space.
    fn hit_test(&self, ctx: &mut HitTestContext, position: Offset) -> bool {
        let mut hit = false;

        if ctx.size.contains(position) {
            while let Some(mut child) = ctx.iter_children().next_back() {
                if child.hit_test(position) == HitTest::Absorb {
                    hit = true;

                    break;
                }
            }
        }

        // if hit {
        //     ctx.add_result(HitTestEntry {
        //         element_id: ctx.element_id,
        //         position,
        //     });
        // }

        hit
    }
}
