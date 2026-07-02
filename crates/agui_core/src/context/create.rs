use std::rc::Rc;

use crate::context::BuildCtx;
use crate::pipeline::render_pipeline::{
    CompositingBitsHook, DeferredSemanticsScope, LayoutBoundaryHandle, LayoutScope, LayoutState,
    PaintBoundaryHandle, PaintScope, PaintState, RelayoutHook, RepaintHook,
    SemanticsBoundaryHandle, SemanticsRebuild, SemanticsScope, SemanticsState,
};
use crate::provide::ProvideScope;
use crate::tree::NodeHandle;

/// The context passed to a widget's `create`: the values in scope and the render channels, but no cursor.
/// `create` builds the element and its render object before the tree exists, so it registers no element-tree
/// node. It may plant a render tree in the forest: a view registers its boundaries here. Reconcile-time grafts
/// derive one from an [`UpdateCtx`](crate::context::UpdateCtx).
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

    /// The repaint boundary registry, for a view to build its repaint hooks against.
    pub fn paint_state(&self) -> &Rc<PaintState> {
        self.paint
    }

    /// A deferred marker for the enclosing semantics boundary, captured by a render object that marks its
    /// semantics from outside a pass, such as an animation.
    #[must_use]
    pub fn deferred_semantics_scope(&self) -> DeferredSemanticsScope {
        self.semantics.deferred_scope(self.semantics_scope)
    }

    /// Registers the semantics boundary at the root of a view's render tree, returning the handle that owns
    /// it. A view registers its root boundary this way at `create`.
    pub fn register_semantics_boundary(
        &self,
        rebuild: SemanticsRebuild,
    ) -> SemanticsBoundaryHandle {
        self.semantics.register(rebuild)
    }

    /// Runs `f` with `semantics` as the enclosing semantics boundary, restoring the previous one afterward. A
    /// view wraps creating its subtree this way so descendants capture its boundary.
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

    /// Registers `relayout` as the relayout boundary at the root of a view's render tree, and returns the
    /// handle that owns it. A view plants its tree's relayout root this way at `create`; it is a forest root,
    /// so its enclosing layout scope is detached. Its enclosing repaint boundary is recorded through
    /// [`set_paint_scope`](crate::pipeline::render_pipeline::LayoutBoundaryHandle::set_paint_scope).
    pub fn register_layout_boundary(&self, relayout: RelayoutHook) -> LayoutBoundaryHandle {
        self.layout.register(LayoutScope::detached(), relayout)
    }

    /// Registers a repaint boundary at the root of a view's render tree, driven by `repaint` and `update_bits`,
    /// and returns the handle that owns it. A view plants its tree's repaint root this way at `create`; it is a
    /// forest root, so its enclosing paint scope is detached. The root paints through the flush, so it is
    /// marked for an initial compositing-bits settle and paint.
    pub fn register_paint_boundary(
        &self,
        repaint: RepaintHook,
        update_bits: CompositingBitsHook,
    ) -> PaintBoundaryHandle {
        let handle = self
            .paint
            .register(PaintScope::detached(), repaint, update_bits);

        handle.mark_needs_compositing_bits_update();

        handle
    }
}
