use std::cell::RefCell;

use agui_core::tree::NodeHandle;

use crate::context::UpdateCtx;
use crate::pipeline::LayoutBuildHost;
use crate::pipeline::render_pipeline::{
    DeferredLayoutScope, LayoutBoundary, LayoutBoundaryHandle, LayoutScope, PaintScope,
    RenderPipeline,
};
use agui_core::scheduling::TaskScheduler;

/// The context threaded through a layout pass: the pipeline to register and mark boundaries against, the
/// relayout boundary in force, and, in a real frame, the element-tree access a `LayoutBuilder` needs to build
/// its child during layout.
pub struct LayoutCtx<'a, 'h> {
    pipeline: &'a RenderPipeline,
    scope: LayoutScope,
    host: Option<&'a RefCell<LayoutBuildHost<'h>>>,
}

impl<'a, 'h> LayoutCtx<'a, 'h> {
    pub fn new(pipeline: &'a RenderPipeline, scope: LayoutScope) -> Self {
        Self {
            pipeline,
            scope,
            host: None,
        }
    }

    /// As [`new`](Self::new), carrying the element-tree access a layout-time build needs. The driver builds one
    /// this way for a real frame, so a `LayoutBuilder` reached during the pass can build its child; a bare
    /// unit-test context from [`new`](Self::new) carries no host.
    pub fn with_host(
        pipeline: &'a RenderPipeline,
        scope: LayoutScope,
        host: &'a RefCell<LayoutBuildHost<'h>>,
    ) -> Self {
        Self {
            pipeline,
            scope,
            host: Some(host),
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
            pipeline: self.pipeline,
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
        &self,
        handle: NodeHandle,
        scheduler: &mut dyn TaskScheduler,
        f: impl FnOnce(&mut UpdateCtx) -> R,
    ) -> Option<R> {
        self.host
            .expect("a layout-time build needs the build host a real frame provides")
            .borrow_mut()
            .build(handle, self.pipeline, scheduler, f)
    }

    /// Registers `boundary` as a relayout boundary nested under the boundary in force, and returns the handle
    /// that owns and marks it. A node that establishes a nested relayout boundary during layout registers it
    /// this way; its enclosing repaint boundary is recorded later during paint.
    pub fn register_layout_boundary(
        &self,
        boundary: Box<dyn LayoutBoundary>,
    ) -> LayoutBoundaryHandle {
        self.pipeline.register_layout_boundary(self.scope, boundary)
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
