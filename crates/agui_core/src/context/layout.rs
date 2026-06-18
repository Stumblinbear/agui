use crate::{
    context::MountCtx,
    pipeline::{
        BoundaryContent,
        layout::{DeferredLayoutScope, LayoutPipeline, LayoutScope, RegisteredLayoutBoundary},
        paint::{PaintPipeline, PaintScope},
    },
};

/// The context threaded through a layout pass.
pub struct LayoutCtx<'a> {
    layout: &'a LayoutPipeline,
    paint: &'a mut PaintPipeline,

    scope: LayoutScope,
}

impl<'a> LayoutCtx<'a> {
    pub fn new(
        layout: &'a LayoutPipeline,
        paint: &'a mut PaintPipeline,
        scope: LayoutScope,
    ) -> Self {
        Self {
            layout,
            paint,

            scope,
        }
    }

    /// The relayout boundary in force. A node forwards this to children and stores it to request a
    /// relayout later.
    pub fn scope(&self) -> &LayoutScope {
        &self.scope
    }

    /// Lays a child out under `scope` as its relayout boundary, against the same paint registry. A
    /// node that establishes a nested relayout boundary lays its child out through this.
    pub fn with_layout_scope<R>(
        &mut self,
        scope: LayoutScope,
        f: impl FnOnce(&mut LayoutCtx) -> R,
    ) -> R {
        let mut child = LayoutCtx {
            paint: self.paint,
            layout: self.layout,

            scope,
        };

        f(&mut child)
    }

    /// Registers `content` as a relayout boundary nested under the boundary in force, enclosed by
    /// `paint`, and returns the handle that owns and marks it. A node that establishes a nested relayout
    /// boundary during layout registers it this way.
    pub fn register_boundary(
        &self,
        content: BoundaryContent,
        paint: PaintScope,
    ) -> RegisteredLayoutBoundary {
        self.layout.register_boundary(self.scope, content, paint)
    }

    /// A deferred handle to the boundary in force, for marking it from a reconcile that holds no context.
    pub fn deferred_layout_scope(&self) -> DeferredLayoutScope {
        self.layout.deferred_scope(self.scope)
    }

    /// Marks `scope`'s boundary for re-layout on the next frame.
    pub fn mark_needs_layout(&self, scope: LayoutScope) {
        self.layout.mark_needs_layout(scope);
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
    pub fn mark_needs_composite(&self, scope: PaintScope) {
        self.paint.mark_needs_composite(scope);
    }

    /// Mounts a subtree built during this layout, painting into `paint_scope`, the boundary the
    /// building node captured at its own mount. Runs `f` with a [`MountCtx`] for that boundary, and
    /// does nothing when detached, since there is no registry to mount into.
    pub fn mount(&mut self, paint_scope: &PaintScope, f: impl FnOnce(&mut MountCtx)) {
        f(&mut MountCtx::new(self.layout, self.paint, paint_scope));
    }
}
