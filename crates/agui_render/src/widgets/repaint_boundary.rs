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
        // unmounts, so the handle never resolves the child after it is gone.
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
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use peniko::{Color, Fill};
    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        paint::{command::PaintCommand, scene::Scene},
        test_harness::TestCtx,
    };

    use super::*;

    /// A leaf that fills a square, counts its paints, and captures the repaint boundary it paints into, so a
    /// test can mark that boundary and confirm the marked boundary repaints through its resolved handle.
    struct Painter {
        paints: Rc<Cell<usize>>,
        scope: Rc<RefCell<Option<DeferredPaintScope>>>,
    }

    impl Widget for Painter {
        type Element = LeafElement<RenderPainter>;

        type Render = RenderPainter;

        fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
            LeafElement::new(RenderPainter {
                paints: self.paints,
                scope: self.scope,
            })
        }

        fn update(self, _ctx: &mut UpdateCtx, _element: &mut Self::Element) {}
    }

    struct RenderPainter {
        paints: Rc<Cell<usize>>,
        scope: Rc<RefCell<Option<DeferredPaintScope>>>,
    }

    impl RenderObject for RenderPainter {
        fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
            d.node_for::<Self>().finish()
        }
    }

    impl RenderBox for RenderPainter {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn measure(&self, _: BoxConstraints) -> Size {
            Size::new(10.0, 10.0)
        }

        fn layout(&mut self, _: &mut LayoutCtx, _: BoxConstraints) -> Size {
            Size::new(10.0, 10.0)
        }

        fn measure_baseline(
            &self,
            _: BoxConstraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn update_compositing_bits(&mut self) -> bool {
            false
        }

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            *self.scope.borrow_mut() = Some(ctx.deferred_paint_scope());
            self.paints.set(self.paints.get() + 1);

            let mut canvas = ctx.canvas();
            let brush = canvas.brush(Color::BLACK);
            canvas.fill(Fill::NonZero, brush, &(offset & Size::new(10.0, 10.0)));
        }
    }

    fn fills(scene: &Scene) -> usize {
        scene
            .commands()
            .iter()
            .filter(|command| matches!(command, PaintCommand::Fill { .. }))
            .count()
    }

    /// Marking the boundary repaints its subtree through the deferred handle the pipeline resolves in
    /// `flush_paint`. Run under Miri, this exercises that resolution for soundness.
    #[test]
    fn a_marked_boundary_repaints_through_its_resolved_handle() {
        let paints = Rc::new(Cell::new(0));
        let scope = Rc::new(RefCell::new(None));

        let widget = RepaintBoundary::new(Painter {
            paints: Rc::clone(&paints),
            scope: Rc::clone(&scope),
        });

        let (owner, view) = TestCtx::new().mount_view(widget);
        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        owner.flush_paint();

        let first = view.composite_frame().rasterize();
        assert_eq!(paints.get(), 1);
        assert_eq!(
            fills(&first),
            1,
            "the boundary's retained layer carries the fill"
        );

        // Mark the boundary through the scope its child captured, then repaint: the pipeline resolves the
        // boundary's deferred handle and repaints its subtree, leaving the rest of the tree alone.
        scope
            .borrow()
            .clone()
            .expect("the boundary is mounted")
            .mark_needs_paint();
        // A deferred mark is applied by `drain_deferred`, which runs in `flush_layout`, so a frame drains
        // before it repaints.
        owner.flush_layout();
        owner.flush_paint();

        let second = view.composite_frame().rasterize();
        assert_eq!(
            paints.get(),
            2,
            "the marked boundary repainted through its handle"
        );
        assert_eq!(fills(&second), 1);
    }
}
