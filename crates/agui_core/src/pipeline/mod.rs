use std::rc::Rc;

use crate::{
    context::MountCtx,
    geometry::Offset,
    input::hit_test::HitTestResult,
    paint::{
        compositing::{Compositor, ContainerLayer, LayerHandle},
        scene::Scene,
    },
    pipeline::{
        layout::{BoundaryContent, LayoutPipeline, LayoutScope},
        paint::{PaintBoundaryHandle, PaintPipeline},
    },
    render_object::{
        RenderObject,
        box_layout::{AnyRenderBox, Constraints, RenderBox},
    },
};

pub mod layout;
pub mod paint;

/// Drives one subtree's whole pipeline: it registers the subtree root as a boundary, lays it out, paints
/// it, composites it for presentation, and hit-tests it.
///
/// The root is the outermost relayout and repaint boundary. Its constraints come from the caller through
/// [`resize`], and every boundary nested inside is re-laid or repainted on its own when only it has
/// changed. The root is mounted when the owner is built and lives for the owner's whole life; [`update`]
/// reconciles it in place.
///
/// [`resize`]: Self::resize
/// [`update`]: Self::update
pub struct PipelineOwner {
    root: BoundaryContent,

    layout: LayoutPipeline,
    paint: PaintPipeline,

    root_layout: LayoutScope,
    root_paint: PaintBoundaryHandle,

    layer: LayerHandle<ContainerLayer>,
}

impl PipelineOwner {
    /// Builds the owner around `root`, registering it as the outermost boundary and mounting its
    /// subtree. The root paints into `layer`, which is composited for presentation.
    pub fn new(mut root: BoundaryContent, layer: LayerHandle<ContainerLayer>) -> Self {
        let (mut paint, root_paint) = PaintPipeline::new(Rc::clone(&root), layer.clone());
        let (layout, root_layout) = LayoutPipeline::new(Rc::clone(&root), root_paint.scope());

        let mut ctx = MountCtx::new(&mut paint, root_paint.scope());
        root.mount(&mut ctx);

        Self {
            root,

            layout,
            paint,

            root_layout,
            root_paint,

            layer,
        }
    }

    pub fn on_needs_layout(&mut self, f: Box<dyn Fn()>) {
        self.layout.on_needs_layout(f);
    }

    pub fn on_needs_paint(&mut self, f: Box<dyn Fn()>) {
        self.paint.on_needs_paint(f);
    }

    /// Reconciles the root render object in place.
    pub fn update(&self, f: impl FnOnce(&mut dyn AnyRenderBox)) {
        f(&mut *self.root.borrow_mut());
    }

    /// Lays the root out under `constraints` and repaints it. The caller drives this on the first frame
    /// and whenever the subtree's outer constraints change.
    pub fn resize(&self, constraints: Constraints) {
        self.root_layout.set_constraints(constraints);
        self.root_paint.mark_needs_paint();
    }

    /// Lays out any boundary that has been marked for layout since the last flush.
    pub fn flush_layout(&mut self) {
        self.layout.flush(&mut self.paint);
    }

    /// Repaints any boundary that has been marked for paint since the last flush.
    pub fn flush_paint(&mut self) {
        self.paint.flush_compositing_bits();

        self.paint.flush_paint();
    }

    /// Composites the subtree's retained layers into a scene to present.
    pub fn composite(&self) -> Scene {
        Compositor::compose(&self.layer)
    }

    /// Hit-tests the subtree at `position`, in the root coordinate space, returning the handlers under
    /// it ordered most-specific first.
    pub fn hit_test(&self, position: Offset) -> HitTestResult {
        let mut result = HitTestResult::new();
        self.root.hit_test(&mut result, position);

        result
    }
}
