use crate::{
    paint::compositing::{ContainerLayer, LayerHandle},
    pipeline::{
        layout::BoundaryContent,
        paint::{PaintBoundaryHandle, PaintPipeline, PaintScope},
    },
};

/// The boundary registry handed to a render object while it mounts, for it to register the repaint
/// boundary it establishes and to read the boundary enclosing it.
pub struct MountCtx<'a> {
    paint: &'a mut PaintPipeline,
    paint_scope: PaintScope,
}

impl<'a> MountCtx<'a> {
    pub fn new(paint: &'a mut PaintPipeline, paint_scope: PaintScope) -> Self {
        Self { paint, paint_scope }
    }

    /// Adds a boundary that paints `content` into `layer`, returning the [`PaintBoundaryHandle`] that
    /// owns it.
    pub fn register_boundary(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<ContainerLayer>,
    ) -> PaintBoundaryHandle {
        self.paint.register(content, layer)
    }

    /// Removes a boundary that is leaving the tree.
    pub fn unregister_boundary(&mut self, handle: PaintBoundaryHandle) {
        self.paint.unregister(handle);
    }

    /// The [`PaintScope`] of the nearest enclosing boundary.
    pub fn paint_scope(&self) -> &PaintScope {
        &self.paint_scope
    }

    /// Mounts a subtree with `scope` as its enclosing boundary, restoring the previous scope afterward.
    /// A boundary calls this so its descendants repaint into it rather than into its own parent.
    pub fn with_paint_scope(&mut self, scope: PaintScope, f: impl FnOnce(&mut Self)) {
        let previous = std::mem::replace(&mut self.paint_scope, scope);
        f(self);
        self.paint_scope = previous;
    }
}
