use crate::{
    context::MountCtx,
    pipeline::{
        layout::{LayoutPipeline, LayoutScope},
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

    /// Mounts a subtree built during this layout, painting into `paint_scope`, the boundary the
    /// building node captured at its own mount. Runs `f` with a [`MountCtx`] for that boundary, and
    /// does nothing when detached, since there is no registry to mount into.
    pub fn mount(&mut self, paint_scope: &PaintScope, f: impl FnOnce(&mut MountCtx)) {
        f(&mut MountCtx::new(self.layout, self.paint, paint_scope));
    }
}
