use std::rc::Rc;

use crate::{
    paint::compositing::{LayerHandle, OffsetLayer},
    pipeline::{
        BoundaryContent,
        layout::LayoutPipeline,
        paint::{DeferredPaintScope, PaintBoundaryHandle, PaintPipeline, PaintScope},
    },
    prelude::render_object::LayoutScope,
    view::ViewHandle,
};

pub struct MountCtx<'a> {
    paint: &'a mut PaintPipeline,
    layout: &'a LayoutPipeline,
    paint_scope: &'a PaintScope,
}

impl<'a> MountCtx<'a> {
    pub fn new(
        layout: &'a LayoutPipeline,
        paint: &'a mut PaintPipeline,
        paint_scope: &'a PaintScope,
    ) -> Self {
        Self {
            paint,
            layout,
            paint_scope,
        }
    }

    /// Adds a paint boundary that paints `content` into `layer`, returning the [`PaintBoundaryHandle`] that
    /// owns it.
    pub fn register_paint_boundary(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> PaintBoundaryHandle {
        self.paint
            .register_boundary(*self.paint_scope, content, layer)
    }

    /// Removes a paint boundary that is leaving the tree.
    pub fn unregister_paint_boundary(&mut self, handle: PaintBoundaryHandle) {
        self.paint.unregister(handle);
    }

    /// The [`PaintScope`] of the nearest enclosing boundary.
    pub fn paint_scope(&self) -> &PaintScope {
        self.paint_scope
    }

    /// A deferred handle to the nearest enclosing boundary, for repainting it from a callback that runs
    /// with no context in hand.
    pub fn deferred_paint_scope(&self) -> DeferredPaintScope {
        self.paint.deferred_scope(*self.paint_scope)
    }

    /// Mounts a subtree with `scope` as its enclosing boundary, restoring the previous scope afterward.
    /// A boundary calls this so its descendants repaint into it rather than into its own parent.
    pub fn with_paint_scope(&mut self, scope: &PaintScope, f: impl FnOnce(&mut MountCtx)) {
        f(&mut MountCtx {
            paint: &mut *self.paint,
            layout: self.layout,
            paint_scope: scope,
        });
    }

    /// Registers `content` as a view: the outermost relayout and paint boundary of a subtree painting
    /// into `layer`, returning the [`ViewHandle`] that owns the per-view operations.
    pub fn register_view(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> ViewHandle {
        let paint =
            self.paint
                .register_boundary(*self.paint_scope, Rc::clone(&content), layer.clone());

        let layout = self.layout.register_boundary(
            LayoutScope::detached(),
            Rc::clone(&content),
            paint.scope(),
        );

        ViewHandle::new(content, paint, layout, layer)
    }
}
