use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use typed_floats::{Positive, PositiveFinite};

use agui_core::tree::Slot;

use crate::{
    context::{CreateCtx, PaintCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::Element,
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    paint::compositing::{CompositedFrame, Compositor, LayerHandle, OffsetLayer},
    paint::scene::SceneCapacity,
    pipeline::render_pipeline::{
        CompositingBitsHook, LayoutBoundaryHandle, LayoutScope, PaintBoundaryHandle, RelayoutFn,
        RelayoutHook, RepaintHook, SemanticsBoundaryHandle, SemanticsRebuild, SemanticsScope,
    },
    render_object::{
        LayoutCtx, RenderObject, SingleChildRenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::{MountedChild, RenderNode, RenderObjectPtr},
    },
    semantics::{SemanticsTree, SemanticsTreeBuilder},
    text::TextBaseline,
    widget::{ElementSequence, RenderBoxElement, RenderBoxWrapper, Widget, WidgetSequence},
};

/// The destination a view delivers its freshly built [`SemanticsTree`] to on each semantics flush. Set through
/// [`ViewHandle::on_semantics`]; the default drops the tree.
pub type SemanticsSink = Box<dyn Fn(SemanticsTree)>;

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

    semantics: SemanticsBoundaryHandle,
    /// The sink the semantics flush delivers this view's tree to. Shared with the boundary's rebuild hook, so
    /// setting it here redirects delivery without re-registering.
    semantics_sink: Rc<RefCell<SemanticsSink>>,
}

impl ViewHandle {
    pub(crate) fn new(
        content: Rc<RefCell<RenderView>>,
        layout: LayoutBoundaryHandle,
        paint: PaintBoundaryHandle,
        layer: LayerHandle<OffsetLayer>,
        semantics: SemanticsBoundaryHandle,
        semantics_sink: Rc<RefCell<SemanticsSink>>,
    ) -> Self {
        Self {
            inner: Rc::new(ViewInner {
                content,
                layout,
                paint,
                layer,
                semantics,
                semantics_sink,
            }),
        }
    }

    /// Sets the sink the view delivers its semantics to on each flush, replacing any previous one. An
    /// accessibility driver installs its adapter here to receive the view's tree.
    pub fn on_semantics(&self, sink: impl Fn(SemanticsTree) + 'static) {
        *self.inner.semantics_sink.borrow_mut() = Box::new(sink);
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

    /// Captures the view's subtree as a semantics tree, for assistive technology and tests.
    pub fn semantics(&self) -> SemanticsTree {
        let nodes = self.inner.semantics.build_semantics(|counter| {
            let mut builder = SemanticsTreeBuilder::new(counter);

            self.inner
                .content
                .borrow_mut()
                .child
                .build_semantics(&mut builder);

            builder.finish()
        });

        SemanticsTree::new(nodes)
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

        // The repaint and compositing-bits hooks paint the view's render object into its own layer, keeping the
        // layer and the paint capacity captured against the pipeline the create context lends.
        let paint_state = Rc::downgrade(ctx.paint_state());
        let capacity = Cell::new(SceneCapacity::default());

        let repaint: RepaintHook = {
            let content = Rc::clone(&content);
            let layer = layer.clone();

            Box::new(move |scope| {
                let Some(paint) = paint_state.upgrade() else {
                    return;
                };

                layer.borrow_mut().clear();
                let recorded =
                    PaintCtx::paint_with_capacity(&layer, capacity.get(), &paint, scope, |ctx| {
                        content.borrow_mut().paint(ctx, Offset::ZERO);
                    });
                capacity.set(recorded);
            })
        };

        let update_bits: CompositingBitsHook = {
            let content = Rc::clone(&content);
            // No prior value exists before the first recomputation, so it counts as changed.
            let mut previous = None;

            Box::new(move || {
                let needs = content.borrow_mut().update_compositing_bits();
                previous.replace(needs) != Some(needs)
            })
        };

        let paint = ctx.register_paint_boundary(repaint, update_bits);

        // The relayout boundary re-lays the view through the shared cell at flush, never through the element.
        // Re-borrowing the cell each flush (rather than caching a pointer into it) is what keeps it sound.
        let relayout_content = Rc::clone(&content);
        let relayout: RelayoutHook = Box::new(RelayoutFn(move |ctx: &mut LayoutCtx| {
            relayout_content.borrow_mut().relayout(ctx);
        }));
        let mut layout = ctx.register_layout_boundary(relayout);

        // The view is its own repaint boundary, so it can record its paint scope now.
        layout.set_paint_scope(paint.scope());

        let semantics_sink: Rc<RefCell<SemanticsSink>> = Rc::new(RefCell::new(Box::new(|_| {})));

        let rebuild: SemanticsRebuild = {
            let content = Rc::clone(&content);
            let sink = Rc::clone(&semantics_sink);
            Box::new(move |counter| {
                let mut builder = SemanticsTreeBuilder::new(counter);
                content.borrow_mut().build_semantics(&mut builder);
                (sink.borrow())(SemanticsTree::new(builder.finish()));
            })
        };

        let semantics = ctx.register_semantics_boundary(rebuild);
        let semantics_scope = semantics.scope();

        // The surface holds the handle for the view's life, keeping its boundaries registered until the view
        // unmounts and clears it; the driver reads a clone to drive frames.
        *self.surface.borrow_mut() = Some(ViewHandle::new(
            Rc::clone(&content),
            layout,
            paint,
            layer,
            semantics,
            semantics_sink,
        ));

        // The subtree is created under the view's semantics boundary, so its render objects capture it.
        let child = ctx.with_semantics_scope(semantics_scope, |ctx| {
            Widget::create(RenderBoxWrapper::new(self.child), ctx)
        });

        ViewElement {
            child: Slot::new(child),
            content,
            surface: self.surface,
            semantics: semantics_scope,
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
    semantics: SemanticsScope,
}

impl<C: Element> ViewElement<C> {
    fn update<CV>(&mut self, ctx: &mut UpdateCtx<'_>, child: CV)
    where
        CV: Widget<Element = C>,
    {
        let semantics = self.semantics;

        ctx.with_semantics_scope(semantics, |ctx| {
            // SAFETY: `self.child` is our own slot.
            unsafe {
                ctx.with_child(&mut self.child, |element, ctx| child.update(ctx, element));
            }
        });
    }
}

// SAFETY: mounts and unmounts its single child through the cursor child operations, clearing the render edge
// on unmount; its own render is `()` and never dereferenced.
unsafe impl<C> Element for ViewElement<C>
where
    C: Element<Render = dyn RenderBox>,
{
    type Render = ();

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::unit()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        let semantics = self.semantics;

        let mounted = ctx.with_semantics_scope(semantics, |ctx| {
            // SAFETY: `self.child` is our own slot.
            unsafe { ctx.mount(&mut self.child) }
        });

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

/// A render-less host for several [`View`]s under one pipeline owner, so a single owner drives more than one
/// window. It contributes no render object of its own, and each child view registers and drives its own
/// boundaries independently of the others.
pub struct ViewContainer<L> {
    children: L,
}

impl<L> ViewContainer<L> {
    /// Builds a container hosting `children`, a sequence of [`View`]s.
    pub fn new(children: L) -> Self {
        Self { children }
    }
}

impl<L> Widget for ViewContainer<L>
where
    L: WidgetSequence + 'static,
    L::Elements: 'static,
    L::Renders: 'static,
{
    type Element = ViewContainerElement<L>;

    type Render = ();

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let (children, renders) = self.children.create(ctx);

        ViewContainerElement { children, renders }
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        // The container has no render object, so a grafted child view has no enclosing relayout boundary here.
        // Each view drives its own layout off its handle.
        // SAFETY: the element's child sequence and renders are its own.
        unsafe {
            self.children.update(
                ctx,
                &mut element.children,
                &mut element.renders,
                LayoutScope::detached(),
            );
        }
    }
}

/// The [`Element`] of a [`ViewContainer`], owning the child view elements and their render edges.
pub struct ViewContainerElement<L: WidgetSequence> {
    children: L::Elements,
    renders: L::Renders,
}

// SAFETY: mounts and unmounts its child views through the cursor child operations of its own sequence; its own
// render is `()` and never dereferenced.
unsafe impl<L> Element for ViewContainerElement<L>
where
    L: WidgetSequence + 'static,
    L::Elements: 'static,
    L::Renders: 'static,
{
    type Render = ();

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::unit()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.children` and `self.renders` are our own.
        unsafe {
            self.children.mount(ctx, &mut self.renders);
        }
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: as `mount`.
        unsafe {
            self.children.unmount(ctx);
        }
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.children.describe(d.node_for::<Self>()).finish()
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

impl RenderView {
    /// Re-lays the view's subtree from the constraints the driver last set. The relayout hook the view
    /// registers calls this through the shared cell.
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

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }
}
