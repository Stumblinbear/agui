use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::{LayoutCtx, PaintCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    pipeline::render_pipeline::LayoutScope,
    render_object::{
        RenderObject, SingleChildRenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::{MountedChild, RenderNode},
    },
    semantics::SemanticsTreeBuilder,
    text::TextBaseline,
};

/// A single-child box pass-through kept by an element that grafts its children, so the element has a stable
/// render-tree presence to mark when it swaps a child.
///
/// It contributes nothing of its own to layout, paint, hit-testing, or semantics: it forwards each to its
/// child verbatim. Its one job is to record, each layout, the relayout boundary it sits under, which the
/// owning element reads through [`layout_scope`](Self::layout_scope) at graft time. The anchor outlives the
/// children grafted under it, so that boundary stays valid across a swap, when the new child has not been
/// laid out yet.
pub struct RenderGraft<Child: ?Sized> {
    layout_scope: LayoutScope,
    child: RenderNode<Child>,
}

impl<Child: ?Sized> RenderGraft<Child> {
    pub fn new() -> Self {
        Self {
            layout_scope: LayoutScope::detached(),
            child: RenderNode::new(()),
        }
    }

    /// The relayout boundary this anchor was laid out under at its most recent layout, for the owning element
    /// to mark when it grafts a new child.
    pub fn layout_scope(&self) -> LayoutScope {
        self.layout_scope
    }
}

impl<Child: ?Sized> Default for RenderGraft<Child> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderGraft<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderGraft<Child> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        // Transparent in diagnostics: present the child's node directly rather than wrapping it in one of
        // our own, since the anchor carries nothing worth showing.
        self.child.describe(d)
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderGraft<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.child.measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();
        self.child.layout_and_get_size(ctx, constraints)
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.child.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}
