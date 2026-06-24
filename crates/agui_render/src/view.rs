use std::{cell::RefCell, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::tree::Slot;

use crate::{
    context::{CreateCtx, PaintCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::Element,
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    paint::compositing::{CompositedFrame, Compositor, LayerHandle, OffsetLayer},
    pipeline::{
        BoundaryContent,
        render_pipeline::{LayoutBoundary, LayoutBoundaryHandle, PaintBoundaryHandle},
    },
    render_object::{
        LayoutCtx, RenderObject, SingleChildRenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::{MountedChild, RenderNode, RenderObjectPtr},
    },
    text::TextBaseline,
    widget::{RenderBoxElement, RenderBoxWrapper, Widget},
};

/// A handle to a mounted [`View`], owning the per-view operations a driver performs on the view's
/// render subtree from outside the tree: constraining it, hit-testing it, and compositing it for
/// presentation.
#[derive(Clone)]
pub struct ViewHandle {
    inner: Rc<ViewInner>,
}

struct ViewInner {
    content: Rc<RefCell<RenderView>>,
    layout: LayoutBoundaryHandle,
    /// Held for its `Drop`, which unregisters the view's repaint boundary and discards its layer; nothing
    /// reads it.
    #[allow(dead_code)]
    paint: PaintBoundaryHandle,
    layer: LayerHandle<OffsetLayer>,
}

impl ViewHandle {
    pub(crate) fn new(
        content: Rc<RefCell<RenderView>>,
        layout: LayoutBoundaryHandle,
        paint: PaintBoundaryHandle,
        layer: LayerHandle<OffsetLayer>,
    ) -> Self {
        Self {
            inner: Rc::new(ViewInner {
                content,
                layout,
                paint,
                layer,
            }),
        }
    }

    /// Lays the view out under `constraints` and repaints it. A driver calls this on the first frame and
    /// whenever the surface backing the view changes size.
    pub fn resize(&self, constraints: BoxConstraints) {
        self.inner.content.borrow_mut().set_constraints(constraints);
        self.inner.layout.mark_needs_layout();
    }

    /// Hit-tests the view at `position`, in the view's coordinate space, returning the handlers under it
    /// ordered most-specific first.
    pub fn hit_test(&self, position: Offset) -> HitTestResult {
        let mut result = HitTestResult::new();
        self.inner.content.hit_test(&mut result, position);

        result
    }

    /// Composites the view's retained layers into the frame to present, ordering rasterized content
    /// and the placements for any system-composited surfaces it contains.
    pub fn composite_frame(&self) -> CompositedFrame {
        Compositor::compose(&self.inner.layer)
    }

    /// Captures the view's render tree as a diagnostics snapshot.
    pub fn diagnostics(&self) -> DiagnosticsNode {
        self.inner
            .content
            .borrow()
            .describe(&mut Diagnostics::new())
    }
}

/// A widget that establishes a view: a relayout and repaint boundary whose subtree is constrained,
/// hit-tested, and composited as one unit through the [`ViewHandle`] it surfaces.
///
/// It is the outermost widget of a render subtree a driver presents, and a forest root: it plants its
/// subtree in the pipeline and presents nothing to its own parent. At `create` it registers its boundaries
/// and fills the `surface` slot it was built with, so the driver can read the handle and drive the view's
/// frames. It is transparent to message routing, so wrapping a subtree in it leaves the subtree's addressing
/// unchanged.
pub struct View<Child> {
    child: Child,
    surface: Rc<RefCell<Option<ViewHandle>>>,
}

impl View<()> {
    /// Builds a view that fills `surface` with its [`ViewHandle`] once mounted.
    pub fn new(surface: Rc<RefCell<Option<ViewHandle>>>) -> Self {
        Self { child: (), surface }
    }

    pub fn child<Child>(self, child: Child) -> View<Child> {
        View {
            child,
            surface: self.surface,
        }
    }
}

impl<Child> Widget for View<Child>
where
    Child: Widget,
    Child::Render: RenderBox + Sized + 'static,
{
    type Element = ViewElement<RenderBoxElement<Child::Element>>;

    type Render = ();

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let layer = LayerHandle::new(OffsetLayer::new());

        let content = Rc::new(RefCell::new(RenderView::new()));
        // The repaint boundary holds the render object through the erased shared cell; `Rc::clone` would pin
        // the source type and fail to unsize, so the method form clones and coerces.
        #[allow(clippy::clone_on_ref_ptr)]
        let paint_content: BoundaryContent = content.clone();
        let paint = ctx.register_paint_boundary(paint_content, layer.clone());

        // The relayout boundary re-lays the view through the shared cell at flush, never through the element.
        // Re-borrowing the cell each flush (rather than caching a pointer into it) is what keeps it sound.
        let mut layout = ctx.register_layout_boundary(Box::new(ViewBoundary {
            content: Rc::clone(&content),
        }));

        // The view is its own repaint boundary, so it can record its paint scope now.
        layout.set_paint_scope(paint.scope());

        // The surface holds the handle for the view's life, keeping its boundaries registered until the view
        // unmounts and clears it; the driver reads a clone to drive frames.
        *self.surface.borrow_mut() =
            Some(ViewHandle::new(Rc::clone(&content), layout, paint, layer));

        ViewElement {
            child: Slot::new(Widget::create(RenderBoxWrapper::new(self.child), ctx)),
            content,
            surface: self.surface,
            render: (),
        }
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.update(ctx, RenderBoxWrapper::new(self.child));
    }
}

/// The [`Element`] of a [`View`]. It renders nothing of its own (`Render = ()`); the boundary the driver
/// drives is the [`RenderView`] in `content`, whose edge this element wires to the view's child at mount and
/// clears at unmount.
pub struct ViewElement<C> {
    child: Slot<C>,
    content: Rc<RefCell<RenderView>>,
    surface: Rc<RefCell<Option<ViewHandle>>>,
    render: (),
}

impl<C: Element> ViewElement<C> {
    fn update<CV>(&mut self, ctx: &mut UpdateCtx<'_>, child: CV)
    where
        CV: Widget<Element = C>,
    {
        // SAFETY: `self.child` is our own slot.
        unsafe {
            ctx.with_child(&mut self.child, |element, ctx| child.update(ctx, element));
        }
    }
}

// SAFETY: mounts and unmounts its single child through the cursor child operations, clearing the render edge
// on unmount; its own render is `()` and never dereferenced.
unsafe impl<C> Element for ViewElement<C>
where
    C: Element<Render = dyn RenderBox>,
{
    type Render = ();

    fn render_object_mut(&mut self) -> &mut () {
        &mut self.render
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::dangling()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        let mounted = unsafe { ctx.mount(&mut self.child) };
        self.content.borrow_mut().adopt_child(mounted);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // Clear the edge before the child's render is freed: the driver may still hold a `ViewHandle` to this
        // boundary, and its next access must find no child rather than a dangling edge.
        self.content.borrow_mut().clear_child();

        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };

        *self.surface.borrow_mut() = None;
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.get().describe(d))
            .finish()
    }
}

/// The view's entry in the relayout registry, owned by it: at flush it re-borrows the shared cell and re-lays
/// the [`RenderView`]. Holding an `Rc` clone (rather than a pointer into the cell) is what keeps it sound, and
/// keeps the render object alive as long as the registration.
struct ViewBoundary {
    content: Rc<RefCell<RenderView>>,
}

impl LayoutBoundary for ViewBoundary {
    fn relayout(&mut self, ctx: &mut LayoutCtx) {
        self.content.borrow_mut().relayout(ctx);
    }
}

/// The render object of a [`View`]: the relayout and repaint boundary at the root of the view's render tree.
/// It holds an edge to the view's root render, which the [`View`]'s child element owns, and forwards layout,
/// paint, and hit-testing through it. It is re-laid from the constraints the driver set, reached through the
/// shared cell the [`ViewHandle`] holds, never the element.
pub struct RenderView {
    child: RenderNode<dyn RenderBox>,
    /// The constraints the driver last sized the view under, re-laid from on an isolated relayout. `None`
    /// until the first `resize`.
    constraints: Option<BoxConstraints>,
}

impl RenderView {
    fn new() -> Self {
        Self {
            child: RenderNode::new(()),
            constraints: None,
        }
    }

    fn set_constraints(&mut self, constraints: BoxConstraints) {
        self.constraints = Some(constraints);
    }

    fn clear_child(&mut self) {
        self.child.clear();
    }
}

impl LayoutBoundary for RenderView {
    fn relayout(&mut self, ctx: &mut LayoutCtx) {
        if let Some(constraints) = self.constraints {
            self.child.layout_and_get_size(ctx, constraints);
        }
    }
}

impl SingleChildRenderObject for RenderView {
    type Child = dyn RenderBox;

    fn adopt_child(&mut self, child: MountedChild<dyn RenderBox>) {
        self.child.set(child);
    }
}

impl RenderObject for RenderView {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl RenderBox for RenderView {
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
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        geometry::Size,
        render_object::box_layout::BoxConstraints,
        test_fixtures::RecordingBox,
        test_harness::{RawWidget, TestCtx},
    };

    #[test]
    fn resize_lays_out_and_paints_the_view_child() {
        let render = RecordingBox::new();
        let laid_out = Rc::clone(&render.laid_out);
        let paints = Rc::clone(&render.paints);

        let mut ctx = TestCtx::new();
        let (owner, view) = ctx.mount_view(RawWidget::new(render));

        view.resize(BoxConstraints::tight(Size::new(120.0_f32, 80.0)));
        owner.flush_layout();
        owner.flush_paint();

        assert_eq!(
            laid_out.get(),
            Some(Size::new(120.0_f32, 80.0)),
            "the view drove its child's layout"
        );
        assert!(paints.get() >= 1, "the view drove its child's paint");

        // Compositing after a frame produces a presentable frame without panicking.
        let _frame = view.composite_frame();
    }

    #[test]
    fn resizing_again_relays_the_child() {
        let render = RecordingBox::new();
        let laid_out = Rc::clone(&render.laid_out);

        let mut ctx = TestCtx::new();
        let (owner, view) = ctx.mount_view(RawWidget::new(render));

        view.resize(BoxConstraints::tight(Size::new(100.0_f32, 100.0)));
        owner.flush_layout();
        assert_eq!(laid_out.get(), Some(Size::new(100.0_f32, 100.0)));

        view.resize(BoxConstraints::tight(Size::new(50.0_f32, 200.0)));
        owner.flush_layout();
        assert_eq!(
            laid_out.get(),
            Some(Size::new(50.0_f32, 200.0)),
            "the view re-laid its child under the new constraints"
        );
    }

    #[test]
    fn resizing_repaints_through_the_layout_coupling() {
        let render = RecordingBox::new();
        let paints = Rc::clone(&render.paints);

        let mut ctx = TestCtx::new();
        let (owner, view) = ctx.mount_view(RawWidget::new(render));

        view.resize(BoxConstraints::tight(Size::new(100.0_f32, 100.0)));
        owner.flush_layout();
        owner.flush_paint();
        let after_first = paints.get();
        assert!(after_first >= 1, "the first frame painted the child");

        // A later resize marks only layout; re-laying the view must mark its own repaint boundary, so the
        // child repaints without `resize` touching paint.
        view.resize(BoxConstraints::tight(Size::new(50.0_f32, 200.0)));
        owner.flush_layout();
        owner.flush_paint();
        assert!(
            paints.get() > after_first,
            "re-laying the view repainted it via the layout-to-paint coupling"
        );
    }

    #[test]
    fn hit_testing_an_empty_view_is_harmless() {
        let mut ctx = TestCtx::new();
        let (owner, view) = ctx.mount_view(RawWidget::new(RecordingBox::new()));

        view.resize(BoxConstraints::tight(Size::new(64.0_f32, 64.0)));
        owner.flush_layout();

        // `RecordingBox` passes hits through, so nothing absorbs; the call must still resolve cleanly.
        let result = view.hit_test(crate::geometry::Offset::new(10.0_f32, 10.0));
        assert!(result.path().is_empty());
    }

    #[test]
    fn marking_a_child_boundary_relays_only_it() {
        let render = RecordingBox::new();
        let layouts = Rc::clone(&render.layouts);
        let boundary = Rc::clone(&render.boundary);

        let mut ctx = TestCtx::new();
        let (owner, view) = ctx.mount_view(RawWidget::new(render));

        view.resize(BoxConstraints::tight(Size::new(40.0_f32, 40.0)));
        owner.flush_layout();
        assert_eq!(layouts.get(), 1, "the first frame laid the child out once");

        // The child is under tight constraints, so it is its own relayout boundary. Marking that boundary and
        // flushing re-lays it without resizing the view above it.
        boundary
            .borrow()
            .as_ref()
            .expect("the child captured its boundary during layout")
            .mark_needs_layout();
        owner.flush_layout();
        assert_eq!(
            layouts.get(),
            2,
            "marking the child's own boundary re-laid it in isolation"
        );
    }
}
