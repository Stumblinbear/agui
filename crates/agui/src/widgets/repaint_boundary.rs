use typed_floats::{Positive, PositiveFinite};

use crate::{
    paint::compositing::{LayerHandle, OffsetLayer},
    pipeline::render_pipeline::PaintBoundaryHandle,
    prelude::{element::*, render_object::*},
    widget::{RenderBoxElement, RenderBoxWrapper},
};

/// A widget whose subtree paints into its own retained layer.
///
/// A change inside the subtree repaints only the subtree; a change anywhere else leaves the subtree's
/// painting untouched and reuses it.
pub struct RepaintBoundary<Child> {
    child: Child,
}

impl<Child> RepaintBoundary<Child> {
    pub fn new(child: Child) -> Self {
        Self { child }
    }
}

impl<Child> Widget for RepaintBoundary<Child>
where
    Child: Widget,
    Child::Render: RenderBox + Sized + 'static,
    Child::Element: 'static,
{
    type Element = SingleChildElement<RenderBoxElement<Child::Element>, RenderRepaintBoundary>;

    type Render = RenderRepaintBoundary;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            RenderBoxWrapper::new(self.child),
            RenderRepaintBoundary {
                child: RenderNode::new(()),
                layer: LayerHandle::new(OffsetLayer::new()),
                handle: None,
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.update(ctx, RenderBoxWrapper::new(self.child));
    }
}

/// The render object of a [`RepaintBoundary`]: it paints its subtree into a retained layer that is reused
/// until the boundary is marked for repaint.
pub struct RenderRepaintBoundary {
    child: RenderNode<dyn RenderBox>,
    layer: LayerHandle<OffsetLayer>,
    handle: Option<PaintBoundaryHandle>,
}

impl SingleChildRenderObject for RenderRepaintBoundary {
    type Child = dyn RenderBox;

    fn adopt_child(&mut self, child: MountedChild<dyn RenderBox>) {
        self.child.set(child);

        // The child changed, so the boundary registered against the old one is stale; drop the handle and
        // re-register against the new child on the next paint.
        self.handle = None;
    }
}

impl RenderObject for RenderRepaintBoundary {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl RenderBox for RenderRepaintBoundary {
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
        // A boundary always composites into its own layer; its subtree's bits are recomputed on the
        // boundary's own repaint, not here.
        true
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        if self.handle.is_some() {
            ctx.add_layer(self.layer.clone(), offset);
            return;
        }

        // SAFETY: the registration is dropped in `adopt_child` on a child swap and when this render object
        // unmounts, so the duplicate never resolves the child after it is gone, and its hooks borrow the
        // child only during the boundary's isolated flush phases.
        let child = unsafe { self.child.child_handle() };
        let handle = ctx.register_paint_boundary(child, self.layer.clone());

        // Fill the fresh layer in this same pass so it is not blank until the boundary's first isolated
        // repaint; its subtree's compositing bits must settle before that paint.
        self.child.update_compositing_bits();
        ctx.push_boundary_layer(handle.scope(), self.layer.clone(), offset, |ctx| {
            self.child.paint(ctx, Offset::ZERO);
        });

        self.handle = Some(handle);
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }
}
