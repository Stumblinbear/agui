use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use crate::{
    context::{MountCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::SingleChildElement,
    geometry::Offset,
    input::hit_test::HitTestResult,
    paint::{
        compositing::{Compositor, LayerHandle, OffsetLayer},
        scene::Scene,
    },
    pipeline::{
        BoundaryContent,
        layout::RegisteredLayoutBoundary,
        paint::{PaintBoundaryHandle, PaintScope},
    },
    prelude::element::ProtocolTag,
    render_object::{
        RenderObject, SingleChildRenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    widget::Widget,
};

/// A handle to a mounted [`View`], owning the per-view operations a driver performs on the view's
/// render subtree from outside the tree: constraining it, hit-testing it, and compositing it for
/// presentation.
#[derive(Clone)]
pub struct ViewHandle {
    inner: Rc<ViewInner>,
}

struct ViewInner {
    content: BoundaryContent,
    paint: PaintBoundaryHandle,
    layout: RegisteredLayoutBoundary,
    layer: LayerHandle<OffsetLayer>,
}

impl ViewHandle {
    pub(crate) fn new(
        content: BoundaryContent,
        paint: PaintBoundaryHandle,
        layout: RegisteredLayoutBoundary,
        layer: LayerHandle<OffsetLayer>,
    ) -> Self {
        Self {
            inner: Rc::new(ViewInner {
                content,
                paint,
                layout,
                layer,
            }),
        }
    }

    /// The paint scope of the view's boundary, for its subtree to repaint into.
    pub fn scope(&self) -> PaintScope {
        self.inner.paint.scope()
    }

    /// Lays the view out under `constraints` and repaints it. A driver calls this on the first frame and
    /// whenever the surface backing the view changes size.
    pub fn resize(&self, constraints: BoxConstraints) {
        self.inner.layout.set_constraints(constraints);
        self.inner.paint.mark_needs_paint();
    }

    /// Hit-tests the view at `position`, in the view's coordinate space, returning the handlers under it
    /// ordered most-specific first.
    pub fn hit_test(&self, position: Offset) -> HitTestResult {
        let mut result = HitTestResult::new();
        self.inner.content.hit_test(&mut result, position);

        result
    }

    /// Composites the view's retained layers into a scene to present.
    pub fn composite(&self) -> Scene {
        Compositor::compose(&self.inner.layer)
    }

    /// Composites the view's retained layers into `scene` to present, replacing its previous content. A
    /// driver presenting every frame composites into one held scene to reuse its storage.
    pub fn composite_into(&self, scene: &mut Scene) {
        Compositor::compose_into(&self.inner.layer, scene);
    }

    /// Captures the view's render tree as a diagnostics snapshot.
    pub fn diagnostics(&self) -> DiagnosticsNode {
        self.inner
            .content
            .borrow()
            .dyn_describe(&mut Diagnostics::new())
    }
}

/// A widget that establishes a view: a relayout and repaint boundary whose subtree is constrained,
/// hit-tested, and composited as one unit through the [`ViewHandle`] it surfaces.
///
/// It is the outermost widget of a render subtree a driver presents. At mount it fills the `surface`
/// slot it was built with, so the driver can read the handle and drive the view's frames. It is
/// transparent to message routing, so wrapping a subtree in it leaves the subtree's addressing
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
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderView<Child::Render>>;

    type Render = RenderView<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderView {
                content: Rc::new(RefCell::new(child_render)),

                layer: LayerHandle::new(OffsetLayer::new()),

                surface: self.surface,

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
            .expect("a mounted view holds its handle")
            .scope();

        // Descendants repaint into this view, not into the boundary above it.
        ctx.with_paint_scope(&scope, |ctx| {
            render_object.with_child_mut(|child_render| {
                element.update(self.child, child_render, ctx);
            });
        });
    }
}

/// The render object of a [`View`]: it registers the view's boundaries at mount, paints its subtree
/// into a retained layer, and surfaces the [`ViewHandle`] the driver presents it through.
pub struct RenderView<Child> {
    content: BoundaryContent,

    layer: LayerHandle<OffsetLayer>,

    surface: Rc<RefCell<Option<ViewHandle>>>,

    handle: Option<ViewHandle>,

    _phantom: PhantomData<fn() -> Child>,
}

impl<Child: RenderBox> SingleChildRenderObject for RenderView<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        let content = self.content.borrow();
        let child_render = content
            .as_any()
            .downcast_ref::<Child>()
            .expect("a view's content keeps its child's render type for its whole life");

        f(child_render)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        let mut content = self.content.borrow_mut();
        let child_render = content
            .as_any_mut()
            .downcast_mut::<Child>()
            .expect("a view's content keeps its child's render type for its whole life");

        f(child_render)
    }
}

impl<Child: RenderBox> RenderObject for RenderView<Child> {
    fn mount(&mut self, ctx: &mut MountCtx) {
        let handle = ctx.register_view(Rc::clone(&self.content), self.layer.clone());
        *self.surface.borrow_mut() = Some(handle.clone());

        // Descendants repaint into this view, not into the boundary above it.
        let mut content = Rc::clone(&self.content);
        let scope = handle.scope();
        ctx.with_paint_scope(&scope, |ctx| content.mount(ctx));

        self.handle = Some(handle);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.content.unmount(ctx);

        // Drop both clones so the boundaries unregister once the driver releases its own.
        self.handle = None;
        *self.surface.borrow_mut() = None;
    }

    fn update_compositing_bits(&mut self) -> bool {
        // A view contributes no rendering of its own.
        false
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child_in(Some(ProtocolTag::BOX), |d| {
                self.content.borrow().dyn_describe(d)
            })
            .finish()
    }
}
