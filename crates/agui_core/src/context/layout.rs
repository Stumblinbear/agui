use crate::{
    context::MountCtx,
    pipeline::{
        layout::LayoutScope,
        paint::{PaintPipeline, PaintScope},
    },
};

/// The context threaded through a layout pass.
pub struct LayoutCtx<'a> {
    scope: LayoutScope,
    paint: Option<&'a mut PaintPipeline>,
}

impl<'a> LayoutCtx<'a> {
    /// A context that reaches no pipeline, for laying a render object out in isolation. Its scope
    /// marks nothing and it mounts nothing.
    pub fn detached() -> LayoutCtx<'static> {
        LayoutCtx {
            scope: LayoutScope::detached(),
            paint: None,
        }
    }

    pub(crate) fn new(scope: LayoutScope, paint: &'a mut PaintPipeline) -> Self {
        Self {
            scope,
            paint: Some(paint),
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
            scope,
            paint: self.paint.as_deref_mut(),
        };

        f(&mut child)
    }

    /// Mounts a subtree built during this layout, painting into `paint_scope`, the boundary the
    /// building node captured at its own mount. Runs `f` with a [`MountCtx`] for that boundary, and
    /// does nothing when detached, since there is no registry to mount into.
    pub fn mount(&mut self, paint_scope: &PaintScope, f: impl FnOnce(&mut MountCtx)) {
        if let Some(paint) = self.paint.as_deref_mut() {
            f(&mut MountCtx::new(paint, paint_scope.clone()));
        }
    }
}
