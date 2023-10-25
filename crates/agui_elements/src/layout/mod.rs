use agui_core::{
    unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size},
    widget::Widget,
};

mod context;
mod instance;

pub use context::*;
pub use instance::*;

pub trait WidgetLayout: 'static {
    fn children(&self) -> Vec<Widget>;

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
    /// The given position will be in the widget's local coordinate space, not the global
    /// coordinate space.
    fn hit_test(&self, ctx: &mut HitTestContext, position: Offset) -> HitTest {
        if ctx.size().contains(position) {
            while let Some(mut child) = ctx.iter_children().next_back() {
                let offset = position - child.offset();

                if child.hit_test_with_offset(offset, position) == HitTest::Absorb {
                    return HitTest::Absorb;
                }
            }
        }

        HitTest::Pass
    }
}
