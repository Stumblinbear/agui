use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    paint::compositing::{LayerHandle, OffsetLayer},
    pipeline::{BoundaryContent, paint::PaintBoundaryHandle},
    prelude::{element::*, render_object::*},
};

/// A widget whose subtree paints into its own retained layer.
///
/// A change inside the subtree repaints only the subtree; a change anywhere else leaves the subtree's
/// painting untouched and reuses it.
pub struct RepaintBoundary<Child> {
    child: Child,
}

impl RepaintBoundary<()> {
    pub fn new() -> Self {
        Self { child: () }
    }

    pub fn child<Child>(self, child: Child) -> RepaintBoundary<Child> {
        RepaintBoundary { child }
    }
}

impl Default for RepaintBoundary<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Child> Widget for RepaintBoundary<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderRepaintBoundary<Child::Render>>;

    type Render = RenderRepaintBoundary<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderRepaintBoundary {
                content: Rc::new(RefCell::new(child_render)),

                layer: LayerHandle::new(OffsetLayer::new()),

                handle: None,

                _phantom: PhantomData,
            },
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        let scope = render_object
            .handle
            .as_ref()
            .expect("a mounted boundary holds its handle")
            .scope();

        // A render object grafted during the rebuild mounts into this boundary, not the one above it.
        ctx.with_paint_scope(&scope, |ctx| {
            render_object.with_child_mut(|child_render| {
                element.update(self.child, child_render, ctx);
            });
        });
    }
}

/// The render object of a [`RepaintBoundary`]: it paints its subtree into a retained layer that is
/// reused until the boundary is marked for repaint.
pub struct RenderRepaintBoundary<Child> {
    content: BoundaryContent,

    layer: LayerHandle<OffsetLayer>,

    handle: Option<PaintBoundaryHandle>,

    _phantom: PhantomData<fn() -> Child>,
}

impl<Child: RenderBox> SingleChildRenderObject for RenderRepaintBoundary<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        let content = self.content.borrow();
        let child_render = content
            .as_any()
            .downcast_ref::<Child>()
            .expect("a boundary's content keeps its child's render type for its whole life");

        f(child_render)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        let mut content = self.content.borrow_mut();
        let child_render = content
            .as_any_mut()
            .downcast_mut::<Child>()
            .expect("a boundary's content keeps its child's render type for its whole life");

        f(child_render)
    }
}

impl<Child: RenderBox> RenderObject for RenderRepaintBoundary<Child> {
    fn mount(&mut self, ctx: &mut MountCtx) {
        let handle = ctx.register_paint_boundary(Rc::clone(&self.content), self.layer.clone());

        // Descendants repaint into this boundary, not into the one above it.
        let mut content = Rc::clone(&self.content);
        let boundary_scope = handle.scope();
        ctx.with_paint_scope(&boundary_scope, |ctx| content.mount(ctx));

        self.handle = Some(handle);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.content.unmount(ctx);

        if let Some(handle) = self.handle.take() {
            ctx.unregister_paint_boundary(handle);
        }
    }

    fn update_compositing_bits(&mut self) -> bool {
        // A boundary always composites into its own layer; its subtree recomputes on its own repaint.
        true
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.content.borrow().dyn_describe(d))
            .finish()
    }
}

impl<Child: RenderBox> RenderBox for RenderRepaintBoundary<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.content.measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.content.layout(ctx, constraints)
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.content.measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.content.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.content.hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        ctx.add_layer(self.layer.clone(), offset);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use agui_core::{
        paint::{
            command::PaintCommand,
            compositing::{Compositor, LayerHandle, OffsetLayer},
            peniko::{Color, Fill},
            scene::Scene,
        },
        pipeline::{BoundaryContent, layout::LayoutPipeline, paint::PaintPipeline},
        prelude::{element::*, render_object::*},
        test_harness::TestCtx,
    };

    use typed_floats::{PositiveFinite, as_const};

    use super::*;

    /// A widget that counts its paints, so a test can see which boundaries repaint.
    struct Counter<Child> {
        paints: Rc<Cell<usize>>,
        capture: Option<Rc<RefCell<Option<DeferredPaintScope>>>>,
        child: Child,
    }

    impl Counter<()> {
        fn new(paints: Rc<Cell<usize>>) -> Self {
            Self {
                paints,
                capture: None,
                child: (),
            }
        }
    }

    impl<Child> Counter<Child> {
        fn child<C>(self, child: C) -> Counter<C> {
            Counter {
                paints: self.paints,
                capture: self.capture,
                child,
            }
        }

        /// Records a deferred handle to this node's enclosing boundary at mount, so a test can mark it.
        fn capture(mut self, slot: Rc<RefCell<Option<DeferredPaintScope>>>) -> Self {
            self.capture = Some(slot);
            self
        }
    }

    impl<Child> Widget for Counter<Child>
    where
        Child: Widget,
        Child::Render: RenderBox,
    {
        type Element = SingleChildElement<Child::Element, RenderCounter<Child::Render>>;
        type Render = RenderCounter<Child::Render>;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let (element, child_render) = SingleChildElement::new(self.child, ctx);
            let render = RenderCounter {
                paints: self.paints,
                capture: self.capture,
                child: RenderNode::new(child_render),
            };
            (element, render)
        }

        fn update(
            self,
            element: &mut Self::Element,
            render_object: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            element.update(self.child, &mut render_object.child.object, ctx);
        }
    }

    struct RenderCounter<C> {
        paints: Rc<Cell<usize>>,
        capture: Option<Rc<RefCell<Option<DeferredPaintScope>>>>,
        child: RenderNode<C>,
    }

    impl<C> SingleChildRenderObject for RenderCounter<C> {
        type Child = C;

        fn with_child<R>(&self, f: impl FnOnce(&C) -> R) -> R {
            f(&self.child.object)
        }

        fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut C) -> R) -> R {
            f(&mut self.child.object)
        }
    }

    impl<C: RenderBox> RenderObject for RenderCounter<C> {
        fn mount(&mut self, ctx: &mut MountCtx) {
            if let Some(slot) = &self.capture {
                *slot.borrow_mut() = Some(ctx.deferred_paint_scope());
            }
            self.child.mount(ctx);
        }

        fn unmount(&mut self, ctx: &mut MountCtx) {
            self.child.unmount(ctx);
        }

        fn update_compositing_bits(&mut self) -> bool {
            self.child.update_compositing_bits()
        }
    }

    impl<C: RenderBox> RenderBox for RenderCounter<C> {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }
        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }
        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }
        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }
        fn measure(&self, _: BoxConstraints) -> Size {
            Size::new(10.0, 10.0)
        }
        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            self.child.layout(ctx, constraints);
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
        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.paints.set(self.paints.get() + 1);

            {
                let mut canvas = ctx.canvas();
                let brush = canvas.brush(Color::BLACK);
                canvas.fill(Fill::NonZero, brush, &(offset & Size::new(10.0, 10.0)));
            }

            self.child.paint(ctx, offset);
        }
    }

    fn fills(scene: &Scene) -> usize {
        scene
            .flatten()
            .commands()
            .iter()
            .filter(|command| matches!(command, PaintCommand::Fill { .. }))
            .count()
    }

    /// Built and mounted through the real pipeline: an outer boundary whose content draws and hosts an
    /// inner boundary. Marking the inner boundary repaints only its content; the outer boundary's content
    /// is not repainted, yet the composed scene still contains both.
    #[test]
    fn marking_an_inner_boundary_leaves_the_outer_one_alone() {
        let outer_paints = Rc::new(Cell::new(0));
        let inner_paints = Rc::new(Cell::new(0));
        let inner_scope = Rc::new(RefCell::new(None));

        let widget =
            Counter::new(Rc::clone(&outer_paints))
                .child(RepaintBoundary::new().child(
                    Counter::new(Rc::clone(&inner_paints)).capture(Rc::clone(&inner_scope)),
                ));

        let (mut owner, view) = TestCtx::new().mount_view(widget);
        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();

        owner.flush_paint();
        let first = view.composite_frame().rasterize();
        assert_eq!(outer_paints.get(), 1);
        assert_eq!(inner_paints.get(), 1);
        assert_eq!(fills(&first), 2, "both boundaries contributed a fill");

        // Reach the inner boundary the way a widget under it does: through the scope handed to it at mount.
        let inner = inner_scope
            .borrow()
            .clone()
            .expect("the inner boundary is mounted");
        inner.mark_needs_paint();

        owner.flush_paint();
        let second = view.composite_frame().rasterize();
        assert_eq!(inner_paints.get(), 2, "the marked inner boundary repainted");
        assert_eq!(
            outer_paints.get(),
            1,
            "the outer boundary's content was not repainted"
        );
        assert_eq!(
            fills(&second),
            2,
            "the outer boundary still embeds the inner one through its retained layer"
        );
    }

    /// A rebuild that changes nothing the subtree draws does not repaint the boundary: with no
    /// descendant marking the boundary's scope, the retained layer is reused.
    #[test]
    fn an_unchanged_rebuild_does_not_repaint_the_boundary() {
        let outer_paints = Rc::new(Cell::new(0));
        let inner_paints = Rc::new(Cell::new(0));

        let widget = Counter::new(Rc::clone(&outer_paints))
            .child(RepaintBoundary::new().child(Counter::new(Rc::clone(&inner_paints))));

        let (mut element, render) = TestCtx::new().create(widget);
        let root = Rc::new(RefCell::new(render));

        // Mount the subtree under a paint boundary by hand, so the rebuild can be driven directly to
        // confirm it leaves the inner boundary's retained layer untouched.
        let root_content: BoundaryContent = root.clone();
        let layer = LayerHandle::new(OffsetLayer::new());
        let (mut pipeline, root_boundary) = PaintPipeline::new(root_content, layer.clone());
        let layout = LayoutPipeline::default();
        {
            let scope = root_boundary.scope();
            let mut ctx = MountCtx::new(&layout, &mut pipeline, &scope);
            root.borrow_mut().mount(&mut ctx);
        }
        root.borrow_mut().layout(
            &mut LayoutCtx::new(&layout, &mut pipeline, LayoutScope::detached()),
            BoxConstraints::new(0, 100, 0, 100),
        );

        pipeline.flush_compositing_bits();
        pipeline.flush_paint();
        let _ = Compositor::compose(&layer);
        assert_eq!(inner_paints.get(), 1);

        // Rebuild with an identical tree, so nothing inside marks the boundary's scope.
        let widget = Counter::new(Rc::clone(&outer_paints))
            .child(RepaintBoundary::new().child(Counter::new(Rc::clone(&inner_paints))));
        TestCtx::new().run(|ctx| {
            let mut render = root.borrow_mut();
            widget.update(&mut element, &mut render, ctx);
        });

        pipeline.flush_compositing_bits();
        pipeline.flush_paint();
        let _ = Compositor::compose(&layer);

        assert_eq!(
            inner_paints.get(),
            1,
            "an unchanged rebuild reuses the boundary's painting instead of repainting it"
        );
    }
}

#[cfg(test)]
mod harness {
    use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

    use super::RepaintBoundary;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_element_lifecycle() {
        ElementLifecycleCheck::new().single_child(|child| RepaintBoundary::new().child(child));
    }

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(|| RepaintBoundary::new().child(SizedBox::new().width(20).height(10)));
    }
}
