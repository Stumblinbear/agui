use crate::pipeline::{
    BoundaryContent,
    render_pipeline::{
        DeferredLayoutScope, LayoutScope, PaintScope, RegisteredLayoutBoundary, RenderPipeline,
    },
};

/// The context threaded through a layout pass: the pipeline to register and mark boundaries against, and the
/// relayout boundary currently in force.
pub struct LayoutCtx<'a> {
    pipeline: &'a RenderPipeline,
    scope: LayoutScope,
}

impl<'a> LayoutCtx<'a> {
    pub fn new(pipeline: &'a RenderPipeline, scope: LayoutScope) -> Self {
        Self { pipeline, scope }
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
            pipeline: self.pipeline,
            scope,
        };

        f(&mut child)
    }

    /// Registers `content` as a relayout boundary nested under the boundary in force, enclosed by `paint`,
    /// and returns the handle that owns and marks it. A node that establishes a nested relayout boundary
    /// during layout registers it this way.
    pub fn register_layout_boundary(
        &self,
        content: BoundaryContent,
        paint: PaintScope,
    ) -> RegisteredLayoutBoundary {
        self.pipeline
            .register_layout_boundary(self.scope, content, paint)
    }

    /// A deferred handle to the boundary in force, for marking it from a reconcile that holds no context.
    pub fn deferred_layout_scope(&self) -> DeferredLayoutScope {
        self.pipeline.deferred_layout_scope(self.scope)
    }

    /// Marks `scope`'s boundary for re-layout on the next frame.
    pub fn mark_needs_layout(&self, scope: LayoutScope) {
        self.pipeline.mark_needs_layout(scope);
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.pipeline.mark_needs_paint(scope);
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.pipeline.mark_needs_compositing_bits_update(scope);
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting any boundary.
    pub fn mark_needs_composite(&self) {
        self.pipeline.mark_needs_composite();
    }
}
