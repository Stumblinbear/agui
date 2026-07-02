use std::rc::Rc;

use crate::{pipeline::render_pipeline::LayoutBuildHost, tree::NodeHandle};

use crate::context::UpdateCtx;
use crate::pipeline::render_pipeline::{
    DeferredLayoutScope, LayoutBoundaryHandle, LayoutScope, LayoutState, PaintScope, PaintState,
    RelayoutHook,
};
use crate::scheduling::TaskScheduler;

/// The context threaded through a layout pass: the layout and paint channels to register and mark boundaries
/// against, the relayout boundary in force, and, in a real frame, the element-tree access a `LayoutBuilder`
/// needs to build its child during layout.
pub struct LayoutCtx<'a, 'h> {
    layout: &'a Rc<LayoutState>,
    paint: &'a Rc<PaintState>,
    scope: LayoutScope,
    host: &'a mut LayoutBuildHost<'h>,
}

impl<'a, 'h> LayoutCtx<'a, 'h> {
    /// As [`new`](Self::new), carrying the element-tree access a layout-time build needs. The driver builds one
    /// this way for a real frame, so a `LayoutBuilder` reached during the pass can build its child; a bare
    /// unit-test context from [`new`](Self::new) carries no host.
    pub(crate) fn new(
        layout: &'a Rc<LayoutState>,
        paint: &'a Rc<PaintState>,
        scope: LayoutScope,
        host: &'a mut LayoutBuildHost<'h>,
    ) -> Self {
        Self {
            layout,
            paint,
            scope,
            host,
        }
    }

    /// The relayout boundary in force. A node forwards this to the children it lays out, and stores it to
    /// request a relayout of that boundary later.
    pub fn scope(&self) -> &LayoutScope {
        &self.scope
    }

    /// Lays a child out under `scope` as its relayout boundary. A node that establishes a nested relayout
    /// boundary lays its child out through this.
    pub fn with_layout_scope<R>(
        &mut self,
        scope: LayoutScope,
        f: impl FnOnce(&mut LayoutCtx) -> R,
    ) -> R {
        let mut child = LayoutCtx {
            layout: self.layout,
            paint: self.paint,
            scope,
            host: self.host,
        };

        f(&mut child)
    }

    /// Hands `f` an [`UpdateCtx`] positioned at the element `handle` names, for it to build or reconcile that
    /// element's child subtree. Returns `f`'s result, or `None` if the element is gone.
    ///
    /// # Panics
    /// If this context carries no build host. A real frame always provides one; only a bare unit-test context
    /// from [`new`](Self::new) lacks one.
    pub fn build_child<R>(
        &mut self,
        handle: NodeHandle,
        scheduler: &mut dyn TaskScheduler,
        f: impl FnOnce(&mut UpdateCtx) -> R,
    ) -> Option<R> {
        self.host.build(handle, scheduler, f)
    }

    /// Registers `boundary` as a relayout boundary nested under the boundary in force, and returns the handle
    /// that owns and marks it. A node that establishes a nested relayout boundary during layout registers it
    /// this way; its enclosing repaint boundary is recorded later during paint.
    pub fn register_layout_boundary(&self, relayout: RelayoutHook) -> LayoutBoundaryHandle {
        self.layout.register(self.scope, relayout)
    }

    /// A deferred handle to the boundary in force, for marking it from a reconcile that holds no context.
    pub fn deferred_layout_scope(&self) -> DeferredLayoutScope {
        self.layout.deferred_scope(self.scope)
    }

    /// Marks `scope`'s boundary for re-layout on the next frame.
    pub fn mark_needs_layout(&self, scope: LayoutScope) {
        self.layout.mark(scope.0);
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.paint.mark_needs_paint(scope);
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.paint.mark_needs_compositing_bits_update(scope);
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting any boundary.
    pub fn mark_needs_composite(&self) {
        self.paint.mark_needs_composite();
    }
}
