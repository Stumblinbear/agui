//! A subtree that paints into its own retained layer, repainted independently of its surroundings.

use std::{cell::RefCell, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::SingleChildElement,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::{ContainerLayer, LayerHandle, PaintCtx},
    render_object::{
        BoundaryContent, MountCtx, PaintBoundaryHandle, RenderObject, box_layout::RenderBox,
    },
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    widget::Widget,
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
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderRepaintBoundary;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        SingleChildElement::new(&self.child, ctx)
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.update(&self.child, &old.child, ctx);
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(&self.child, path, action)
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        let child = element.create_render_object(&self.child);

        RenderRepaintBoundary {
            content: Rc::new(RefCell::new(Box::new(child))),
            layer: LayerHandle::new(ContainerLayer::new()),
            handle: None,
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        {
            let mut content = render_object.content.borrow_mut();
            let child = content
                .as_any_mut()
                .downcast_mut::<Child::Render>()
                .expect("a boundary's content keeps its child's render type for its whole life");
            element.update_render_object(&self.child, child);
        }

        // The subtree's description changed, so the boundary must repaint.
        if let Some(handle) = &render_object.handle {
            handle.mark_needs_paint();
        }
    }
}

/// The render object of a [`RepaintBoundary`]: it paints its subtree into a retained layer that is
/// reused until the boundary is marked for repaint.
pub struct RenderRepaintBoundary {
    content: BoundaryContent,
    layer: LayerHandle<ContainerLayer>,
    handle: Option<PaintBoundaryHandle>,
}

impl RenderObject for RenderRepaintBoundary {
    fn mount(&mut self, ctx: &mut MountCtx) {
        let handle = ctx.register_boundary(Rc::clone(&self.content), self.layer.clone());

        // Descendants repaint into this boundary, not into the one above it.
        let content = Rc::clone(&self.content);
        ctx.with_paint_scope(handle.scope(), |ctx| content.borrow_mut().mount(ctx));

        self.handle = Some(handle);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.content.borrow_mut().unmount(ctx);

        if let Some(handle) = self.handle.take() {
            ctx.unregister_boundary(handle);
        }
    }

    fn update_compositing_bits(&mut self) -> bool {
        // A boundary always composites into its own layer; its subtree recomputes on its own repaint.
        true
    }
}

impl RenderBox for RenderRepaintBoundary {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().max_intrinsic_height(width)
    }

    fn measure(&self, constraints: Constraints) -> Size {
        self.content.borrow().measure(constraints)
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        self.content.borrow_mut().layout(constraints)
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.content
            .borrow()
            .measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.content.borrow_mut().distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.content.borrow().hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx) {
        ctx.add_layer(self.layer.clone().into());
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use agui_core::{
        element::SingleChildElement,
        paint::{
            PaintCommand, Scene,
            peniko::{Color, Fill},
        },
        rect::Rect,
        render_object::{PaintScope, RenderNode, RenderOwner},
        test_harness::TestHarness,
    };
    use typed_floats::{PositiveFinite, as_const};

    use super::*;

    /// A widget that counts its paints, so a test can see which boundaries repaint.
    struct Counter<Child> {
        paints: Rc<Cell<usize>>,
        capture: Option<Rc<RefCell<Option<PaintScope>>>>,
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

        /// Records this node's enclosing paint scope at mount, so a test can mark that boundary.
        fn capture(mut self, slot: Rc<RefCell<Option<PaintScope>>>) -> Self {
            self.capture = Some(slot);
            self
        }
    }

    impl<Child> Widget for Counter<Child>
    where
        Child: Widget,
        Child::Render: RenderBox,
    {
        type Element = SingleChildElement<Child::Element>;
        type Render = RenderCounter<Child::Render>;

        fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
            SingleChildElement::new(&self.child, ctx)
        }

        fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
            element.update(&self.child, &old.child, ctx);
        }

        fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
            element.dispatch(&self.child, path, action)
        }

        fn create_render_object(&self, element: &Self::Element) -> Self::Render {
            RenderCounter {
                paints: Rc::clone(&self.paints),
                capture: self.capture.clone(),
                child: RenderNode::new(element.create_render_object(&self.child)),
            }
        }

        fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
            element.update_render_object(&self.child, &mut render_object.child.object);
        }
    }

    struct RenderCounter<C> {
        paints: Rc<Cell<usize>>,
        capture: Option<Rc<RefCell<Option<PaintScope>>>>,
        child: RenderNode<C>,
    }

    impl<C: RenderBox> RenderObject for RenderCounter<C> {
        fn mount(&mut self, ctx: &mut MountCtx) {
            if let Some(slot) = &self.capture {
                *slot.borrow_mut() = Some(ctx.paint_scope().clone());
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
        fn measure(&self, _: Constraints) -> Size {
            Size::new(10.0, 10.0)
        }
        fn layout(&mut self, constraints: Constraints) -> Size {
            self.child.layout(constraints);
            Size::new(10.0, 10.0)
        }
        fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }
        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }
        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }
        fn paint(&mut self, ctx: &mut PaintCtx) {
            self.paints.set(self.paints.get() + 1);

            {
                let mut canvas = ctx.canvas();
                let brush = canvas.brush(Color::BLACK);
                canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(10.0, 10.0)));
            }

            self.child.paint(ctx);
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
        let mut owner = RenderOwner::new();
        let outer_paints = Rc::new(Cell::new(0));
        let inner_paints = Rc::new(Cell::new(0));
        let inner_scope = Rc::new(RefCell::new(None));

        let widget =
            Counter::new(Rc::clone(&outer_paints))
                .child(RepaintBoundary::new().child(
                    Counter::new(Rc::clone(&inner_paints)).capture(Rc::clone(&inner_scope)),
                ));

        let render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
        owner.mount_view(Box::new(render));
        owner.layout(Constraints::new(0, 100, 0, 100));

        owner.flush_paint();
        let first = owner.composite();
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
        let second = owner.composite();
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
}
