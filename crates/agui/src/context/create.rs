use std::rc::Rc;

use agui_core::tree::NodeHandle;

use crate::context::BuildCtx;
use crate::paint::compositing::{LayerHandle, OffsetLayer};
use crate::pipeline::BoundaryContent;
use crate::pipeline::render_pipeline::{
    DeferredSemanticsScope, LayoutBoundary, LayoutBoundaryHandle, LayoutScope, LayoutState,
    PaintBoundaryHandle, PaintContent, PaintScope, PaintState, SemanticsBoundaryHandle,
    SemanticsScope, SemanticsState,
};
use crate::provide::ProvideScope;

/// The context passed to a widget's `create`: the values in scope and the render channels, but no cursor.
/// `create` builds the element and its render object before the tree exists, so it registers no element-tree
/// node. It may plant a render tree in the forest: a [`View`](crate::view::View) registers its boundaries
/// here. Reconcile-time grafts derive one from an [`UpdateCtx`](crate::context::UpdateCtx).
pub struct CreateCtx<'a> {
    provide_scope: ProvideScope,
    layout: &'a Rc<LayoutState>,
    paint: &'a Rc<PaintState>,
    semantics: &'a Rc<SemanticsState>,
    semantics_scope: SemanticsScope,
}

impl<'a> CreateCtx<'a> {
    /// A create context in `scope` against the render channels. The driver builds one to create the root
    /// widget.
    pub(crate) fn new(
        scope: ProvideScope,
        layout: &'a Rc<LayoutState>,
        paint: &'a Rc<PaintState>,
        semantics: &'a Rc<SemanticsState>,
    ) -> Self {
        Self {
            provide_scope: scope,
            layout,
            paint,
            semantics,
            semantics_scope: SemanticsScope::detached(),
        }
    }

    /// Runs `f` with the widget-facing [`BuildCtx`] for this scope, for a stateful widget to compose its
    /// child during `create`. No element handle exists yet, so a dependency read cannot register until mount.
    /// The context lives only for the call.
    pub fn build<R>(&self, f: impl FnOnce(&mut BuildCtx) -> R) -> R {
        f(&mut BuildCtx::new(
            self.provide_scope,
            NodeHandle::default(),
        ))
    }

    /// A deferred marker for the enclosing semantics boundary, captured by a render object that marks its
    /// semantics from outside a pass, such as an animation.
    #[must_use]
    pub fn deferred_semantics_scope(&self) -> DeferredSemanticsScope {
        self.semantics.deferred_scope(self.semantics_scope)
    }

    /// Registers the semantics boundary at the root of a view's render tree, returning the handle that owns
    /// it. A [`View`](crate::view::View) registers its root boundary this way at `create`.
    pub fn register_semantics_boundary(&self, content: BoundaryContent) -> SemanticsBoundaryHandle {
        self.semantics.register(content)
    }

    /// Runs `f` with `semantics` as the enclosing semantics boundary, restoring the previous one afterward. A
    /// [`View`](crate::view::View) wraps creating its subtree this way so descendants capture its boundary.
    pub fn with_semantics_scope<R>(
        &mut self,
        semantics_scope: SemanticsScope,
        f: impl FnOnce(&mut CreateCtx) -> R,
    ) -> R {
        let previous = std::mem::replace(&mut self.semantics_scope, semantics_scope);
        let result = f(self);
        self.semantics_scope = previous;
        result
    }

    /// Registers `boundary` as the relayout boundary at the root of a view's render tree, and returns the
    /// handle that owns it. A [`View`](crate::view::View) plants its tree's relayout root this way at `create`;
    /// it is a forest root, so its enclosing layout scope is detached. Its enclosing repaint boundary is
    /// recorded through [`set_paint_scope`](crate::pipeline::render_pipeline::LayoutBoundaryHandle::set_paint_scope).
    pub fn register_layout_boundary(
        &self,
        boundary: Box<dyn LayoutBoundary>,
    ) -> LayoutBoundaryHandle {
        self.layout.register(LayoutScope::detached(), boundary)
    }

    /// Registers `content` as the repaint boundary at the root of a view's render tree, painting into
    /// `layer`, and returns the handle that owns it. A [`View`](crate::view::View) plants its tree's repaint
    /// root this way at `create`; it is a forest root, so its enclosing paint scope is detached.
    pub fn register_paint_boundary(
        &self,
        content: BoundaryContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> PaintBoundaryHandle {
        let handle =
            self.paint
                .register(PaintScope::detached(), PaintContent::Root(content), layer);

        // The root paints through the flush, so mark it for an initial bits settle and paint.
        handle.mark_needs_compositing_bits_update();

        handle
    }
}
