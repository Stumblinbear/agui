use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use fnv::FnvHashSet;
use slotmap::{SlotMap, new_key_type};

use crate::{
    context::PaintCtx,
    geometry::Offset,
    paint::compositing::{ContainerLayer, LayerHandle},
    pipeline::layout::BoundaryContent,
    render_object::{RenderObject, box_layout::RenderBox},
};

new_key_type! {
    /// Identifies one inner repaint boundary within a [`PaintPipeline`].
    pub struct PaintBoundaryId;
}

/// Owns the repaint boundaries of one subtree and repaints the ones that have changed.
///
/// The subtree's root paints into the layer composited for presentation; every boundary nested inside
/// paints into its own retained layer and can be repainted on its own, leaving every other boundary's
/// layer untouched. Mark the root through [`root_scope`](Self::root_scope), and an inner boundary
/// through the [`PaintScope`] handed back when it is registered.
pub struct PaintPipeline {
    boundaries: SlotMap<PaintBoundaryId, PaintBoundaryState>,
    pending: Rc<RefCell<PaintPipelineState>>,
}

fn noop() {}

impl Default for PaintPipeline {
    fn default() -> Self {
        Self {
            boundaries: SlotMap::with_key(),
            pending: Rc::new(RefCell::new(PaintPipelineState {
                needs_paint: FnvHashSet::default(),
                needs_compositing: FnvHashSet::default(),
                notify: Box::new(noop),

                phase: PaintPipelinePhase::Idle,
            })),
        }
    }
}

impl PaintPipeline {
    /// Registers `root` as a boundary painting into `layer` and returns the pipeline together with the
    /// handle that owns it.
    pub fn new(
        root: BoundaryContent,
        layer: LayerHandle<ContainerLayer>,
    ) -> (Self, PaintBoundaryHandle) {
        let mut pipeline = Self::default();
        let handle = pipeline.register(root, layer);

        (pipeline, handle)
    }

    pub fn on_needs_paint(&mut self, f: Box<dyn Fn()>) {
        self.pending.borrow_mut().notify = f;
    }

    /// Adds a boundary that paints `content` into `layer`, returning a [`PaintBoundaryHandle`] that owns
    /// the boundary and hands out [`PaintScope`]s for marking it.
    ///
    /// # Panics
    ///
    /// Panics if called while updating compositing bits or painting is in progress.
    pub fn register(
        &mut self,
        content: BoundaryContent,
        layer: LayerHandle<ContainerLayer>,
    ) -> PaintBoundaryHandle {
        match self.pending.borrow().phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!("a boundary cannot be registered while updating compositing bits");
            }

            PaintPipelinePhase::Paint => {
                panic!("a boundary cannot be registered while painting is in progress");
            }
        }

        let id = self
            .boundaries
            .insert(PaintBoundaryState { content, layer });

        // A fresh boundary has never been painted and its bits have never been computed.
        {
            let mut pending = self.pending.borrow_mut();
            pending.needs_paint.insert(id);
            pending.needs_compositing.insert(id);
        }

        tracing::debug!(boundary = ?id, "registered repaint boundary");

        PaintBoundaryHandle {
            scope: PaintScope(PaintScopeInner::Boundary {
                id,
                pending: Rc::downgrade(&self.pending),
            }),
        }
    }

    /// Removes a boundary, discarding its layer and clearing any pending mark. The handle is spent.
    ///
    /// # Panics
    ///
    /// Panics if called while a paint pass is in progress.
    // Taking the handle by value spends it, so it cannot mark a boundary that no longer exists.
    #[allow(clippy::needless_pass_by_value)]
    pub fn unregister(&mut self, handle: PaintBoundaryHandle) {
        match self.pending.borrow().phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!("a boundary cannot be unregistered while updating compositing bits");
            }

            PaintPipelinePhase::Paint => {
                panic!("a boundary cannot be unregistered while painting is in progress");
            }
        }

        let PaintScopeInner::Boundary { id, .. } = handle.scope.0 else {
            return;
        };

        self.boundaries
            .remove(id)
            .expect("cannot unregister a boundary that is not registered");

        {
            let mut pending = self.pending.borrow_mut();
            pending.needs_paint.remove(&id);
            pending.needs_compositing.remove(&id);
        }

        tracing::debug!(boundary = ?id, "unregistered repaint boundary");
    }

    /// Recomputes the compositing bits of every marked inner boundary, settling them before any repaint
    /// reads them.
    ///
    /// # Panics
    ///
    /// Panics if called while updating compositing bits or painting is in progress.
    pub fn flush_compositing_bits(&mut self) {
        let mut pending = self.pending.borrow_mut();

        match pending.phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!("compositing bits cannot be recomputed while updating compositing bits");
            }

            PaintPipelinePhase::Paint => {
                panic!("compositing bits cannot be recomputed while painting is in progress");
            }
        }

        if pending.needs_compositing.is_empty() {
            return;
        }

        pending.phase = PaintPipelinePhase::UpdateCompositingBits;

        let ids: Vec<PaintBoundaryId> = pending.needs_compositing.drain().collect();

        tracing::debug!(count = ids.len(), "recomputing compositing bits");

        for id in ids {
            self.boundaries[id].content.update_compositing_bits();
        }

        pending.phase = PaintPipelinePhase::Idle;
    }

    /// Repaints every marked inner boundary into its own layer, leaving the rest as they are.
    ///
    /// # Panics
    ///
    /// Panics if called while updating compositing bits or painting is in progress.
    pub fn flush_paint(&mut self) {
        let mut pending = self.pending.borrow_mut();

        match pending.phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!("paint cannot be flushed while updating compositing bits");
            }

            PaintPipelinePhase::Paint => {
                panic!("paint cannot be flushed while painting is in progress");
            }
        }

        if pending.needs_paint.is_empty() {
            return;
        }

        pending.phase = PaintPipelinePhase::Paint;

        let ids = pending.needs_paint.drain();

        tracing::debug!(count = ids.len(), "flushing paint");

        for id in ids {
            tracing::debug!(boundary = ?id, "repainting boundary");

            let boundary = &mut self.boundaries[id];
            let layer = &boundary.layer;

            layer.borrow_mut().clear();

            PaintCtx::paint(layer, |ctx| {
                boundary.content.paint(ctx, Offset::ZERO);
            });
        }

        pending.phase = PaintPipelinePhase::Idle;
    }
}

enum PaintPipelinePhase {
    Idle,
    UpdateCompositingBits,
    Paint,
}

struct PaintBoundaryState {
    content: BoundaryContent,
    layer: LayerHandle<ContainerLayer>,
}

/// The boundaries awaiting repaint or a bit recompute, and the hook that schedules a frame.
struct PaintPipelineState {
    needs_compositing: FnvHashSet<PaintBoundaryId>,
    needs_paint: FnvHashSet<PaintBoundaryId>,

    notify: Box<dyn Fn()>,

    phase: PaintPipelinePhase,
}

impl PaintPipelineState {
    /// Returns `true` if nothing is awaiting repaint or compositing update.
    fn is_clean(&self) -> bool {
        self.needs_paint.is_empty() && self.needs_compositing.is_empty()
    }

    fn mark_needs_paint(&mut self, id: PaintBoundaryId) {
        match self.phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!("cannot mark a render object for paint while updating compositing bits");
            }

            PaintPipelinePhase::Paint => {
                panic!("cannot mark a render object for paint while painting is in progress");
            }
        }

        tracing::trace!(boundary = ?id, "marked boundary for repaint");

        let was_clean = self.is_clean();

        self.needs_paint.insert(id);

        if was_clean {
            (self.notify)();
        }
    }

    fn mark_needs_compositing_update(&mut self, id: PaintBoundaryId) {
        match self.phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!(
                    "cannot mark a render object for compositing update while updating compositing bits"
                );
            }

            PaintPipelinePhase::Paint => {
                panic!(
                    "cannot mark a render object for compositing update while painting is in progress"
                );
            }
        }

        tracing::trace!(boundary = ?id, "marked boundary for compositing update");

        let was_clean = self.is_clean();

        self.needs_compositing.insert(id);
        self.needs_paint.insert(id);

        if was_clean {
            (self.notify)();
        }
    }
}

/// The boundary a render object repaints into. A node deep in a subtree holds the scope of its nearest
/// enclosing boundary and marks it when its painting goes stale; the boundary then repaints on the next
/// frame, leaving every other boundary untouched. Cloning shares the same target, so the scope can be
/// marked from anywhere, including a per-frame callback. A scope can only mark its boundary, never
/// remove it, so it is safe to hand down a subtree.
#[derive(Clone)]
pub struct PaintScope(PaintScopeInner);

#[derive(Clone)]
enum PaintScopeInner {
    Detached,

    Boundary {
        id: PaintBoundaryId,
        pending: Weak<RefCell<PaintPipelineState>>,
    },
}

impl PaintScope {
    /// A scope detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self(PaintScopeInner::Detached)
    }

    /// Marks the boundary to be repainted on the next frame. If this is the first mark on an otherwise
    /// clean pipeline, the schedule hook fires so the driver schedules a frame.
    pub fn mark_needs_paint(&self) {
        let PaintScopeInner::Boundary { pending, id, .. } = &self.0 else {
            return;
        };

        let Some(pending) = pending.upgrade() else {
            return;
        };

        pending.borrow_mut().mark_needs_paint(*id);
    }

    /// Marks the boundary's compositing bits for recomputation before its next repaint, and the
    /// boundary for repaint. Only an inner boundary tracks its bits this way.
    pub fn mark_needs_compositing_update(&self) {
        let PaintScopeInner::Boundary { pending, id, .. } = &self.0 else {
            return;
        };

        let Some(pending) = pending.upgrade() else {
            return;
        };

        pending.borrow_mut().mark_needs_compositing_update(*id);
    }
}

/// A registered boundary, returned to the render object that registered it. It owns the boundary's
/// place in the pipeline and is the only thing [`unregister`](PaintPipeline::unregister) accepts; it
/// hands out mark-only [`PaintScope`]s for the subtree. Keeping removal here, off the scope, stops a
/// descendant that was handed a scope to mark with from unregistering the boundary it lives under.
pub struct PaintBoundaryHandle {
    scope: PaintScope,
}

impl PaintBoundaryHandle {
    /// A mark-only handle to this boundary, for descendants to repaint into it.
    pub fn scope(&self) -> PaintScope {
        self.scope.clone()
    }

    /// Marks this boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self) {
        self.scope.mark_needs_paint();
    }

    /// Marks this boundary's compositing bits for recomputation before its next repaint.
    pub fn mark_needs_compositing_update(&self) {
        self.scope.mark_needs_compositing_update();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        time::Duration,
    };

    use typed_floats::{Positive, PositiveFinite, as_const};

    use crate::{
        context::{LayoutCtx, MountCtx},
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        paint::{
            command::PaintCommand,
            compositing::Compositor,
            peniko::{Color, Fill},
            scene::Scene,
        },
        render_object::{
            RenderObject,
            box_layout::{AnyRenderBox, BoxConstraints, RenderBox},
        },
        scheduling::Vsync,
        text::TextBaseline,
    };

    use super::*;

    /// A leaf that counts its paints and fills a unit square, so a test can tell whether it repainted.
    struct Counter {
        paints: Rc<Cell<usize>>,
        color: Color,
    }

    /// A boundary content that fills, then embeds the layers of nested boundaries, standing in for a
    /// render object that hosts child repaint boundaries.
    struct Embedder {
        paints: Rc<Cell<usize>>,
        color: Color,
        children: Vec<LayerHandle<ContainerLayer>>,
    }

    macro_rules! trivial_box_layout {
        () => {
            fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
                Some(as_const!(PositiveFinite, f32, 0.0))
            }

            fn measure(&self, _: BoxConstraints) -> Size {
                Size::new(1.0, 1.0)
            }

            fn layout(&mut self, _: &mut LayoutCtx, _: BoxConstraints) -> Size {
                Size::new(1.0, 1.0)
            }

            fn measure_baseline(
                &self,
                _: BoxConstraints,
                _: TextBaseline,
            ) -> Option<PositiveFinite<f32>> {
                None
            }

            fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
                None
            }

            fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
                HitTest::Pass
            }
        };
    }

    impl RenderObject for Counter {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for Counter {
        trivial_box_layout!();

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.paints.set(self.paints.get() + 1);

            let mut canvas = ctx.canvas();
            let brush = canvas.brush(self.color);
            canvas.fill(Fill::NonZero, brush, &(offset & Size::new(1.0, 1.0)));
        }
    }

    impl RenderObject for Embedder {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for Embedder {
        trivial_box_layout!();

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.paints.set(self.paints.get() + 1);

            {
                let mut canvas = ctx.canvas();
                let brush = canvas.brush(self.color);
                canvas.fill(Fill::NonZero, brush, &(offset & Size::new(1.0, 1.0)));
            }

            for child in &self.children {
                ctx.add_layer(child.clone().into(), offset);
            }
        }
    }

    struct Probe {
        paints: Rc<Cell<usize>>,
        bits: Rc<Cell<usize>>,
    }

    impl RenderObject for Probe {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
        fn update_compositing_bits(&mut self) -> bool {
            self.bits.set(self.bits.get() + 1);
            false
        }
    }

    impl RenderBox for Probe {
        trivial_box_layout!();

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {
            self.paints.set(self.paints.get() + 1);
        }
    }

    fn content(render: impl AnyRenderBox + 'static) -> BoundaryContent {
        Rc::new(RefCell::new(render))
    }

    fn layer() -> LayerHandle<ContainerLayer> {
        LayerHandle::new(ContainerLayer::new())
    }

    fn fills(scene: &Scene) -> usize {
        scene
            .flatten()
            .commands()
            .iter()
            .filter(|command| matches!(command, PaintCommand::Fill { .. }))
            .count()
    }

    /// Flushes the inner channels and clears the root, as a frame with a painted root does, against a
    /// freestanding pipeline.
    fn flush_paint(pipeline: &mut PaintPipeline) {
        pipeline.flush_compositing_bits();
        pipeline.flush_paint();
    }

    #[test]
    fn marking_a_boundary_repaints_only_it() {
        let root_paints = Rc::new(Cell::new(0));
        let child_paints = Rc::new(Cell::new(0));

        let root_layer = layer();
        let child_layer = layer();
        let (mut pipeline, _root) = PaintPipeline::new(
            content(Embedder {
                paints: Rc::clone(&root_paints),
                color: Color::BLACK,
                children: vec![child_layer.clone()],
            }),
            root_layer.clone(),
        );
        let child = pipeline.register(
            content(Counter {
                paints: Rc::clone(&child_paints),
                color: Color::rgb8(255, 0, 0),
            }),
            child_layer,
        );

        flush_paint(&mut pipeline);
        let first = Compositor::compose(&root_layer);
        assert_eq!(root_paints.get(), 1);
        assert_eq!(child_paints.get(), 1);
        assert_eq!(
            fills(&first),
            2,
            "the root and child both contributed a fill"
        );

        // The kind of out-of-band mark a per-frame animation callback would make.
        child.mark_needs_paint();

        flush_paint(&mut pipeline);
        let second = Compositor::compose(&root_layer);
        assert_eq!(child_paints.get(), 2, "the marked boundary repainted");
        assert_eq!(
            root_paints.get(),
            1,
            "the clean parent boundary was not repainted"
        );
        assert_eq!(
            fills(&second),
            2,
            "the parent still embeds the child through its retained layer"
        );
    }

    #[test]
    fn a_clean_frame_repaints_nothing() {
        let paints = Rc::new(Cell::new(0));

        let (mut pipeline, _root) = PaintPipeline::new(
            content(Counter {
                paints: Rc::clone(&paints),
                color: Color::BLACK,
            }),
            layer(),
        );

        flush_paint(&mut pipeline);
        flush_paint(&mut pipeline);

        assert_eq!(
            paints.get(),
            1,
            "an unmarked boundary paints once and is reused"
        );
    }

    #[test]
    fn marking_schedules_a_frame_on_the_clean_to_dirty_edge() {
        let frames = Rc::new(Cell::new(0));
        let scheduled = Rc::clone(&frames);
        let (mut pipeline, scope) = PaintPipeline::new(
            content(Counter {
                paints: Rc::new(Cell::new(0)),
                color: Color::BLACK,
            }),
            layer(),
        );
        pipeline.on_needs_paint(Box::new(move || scheduled.set(scheduled.get() + 1)));

        // Registration left the boundary dirty; the first frame clears it without involving the hook.
        flush_paint(&mut pipeline);
        assert_eq!(frames.get(), 0, "registration alone schedules no frame");

        scope.mark_needs_paint();
        scope.mark_needs_paint();
        assert_eq!(
            frames.get(),
            1,
            "only the clean-to-dirty edge schedules a frame"
        );

        flush_paint(&mut pipeline);
        scope.mark_needs_paint();
        assert_eq!(
            frames.get(),
            2,
            "a mark after the flush schedules another frame"
        );
    }

    #[test]
    fn compositing_update_is_a_separate_channel_from_paint() {
        let paints = Rc::new(Cell::new(0));
        let bits = Rc::new(Cell::new(0));

        let (mut pipeline, boundary) = PaintPipeline::new(
            content(Probe {
                paints: Rc::clone(&paints),
                bits: Rc::clone(&bits),
            }),
            layer(),
        );

        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 1);
        assert_eq!(
            bits.get(),
            1,
            "registration computes the bits for the first frame"
        );

        boundary.mark_needs_paint();
        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 2, "the boundary repainted");
        assert_eq!(bits.get(), 1, "a paint mark does not recompute bits");

        boundary.mark_needs_compositing_update();
        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 3, "the boundary repainted");
        assert_eq!(bits.get(), 2, "a compositing mark recomputes bits");
    }

    #[test]
    fn an_animation_repaints_only_its_own_boundary() {
        let vsync = Vsync::new();

        let static_paints = Rc::new(Cell::new(0));
        let animated_paints = Rc::new(Cell::new(0));

        let static_layer = layer();
        let animated_layer = layer();
        let (mut pipeline, _root) = PaintPipeline::new(
            content(Embedder {
                paints: Rc::new(Cell::new(0)),
                color: Color::WHITE,
                children: vec![static_layer.clone(), animated_layer.clone()],
            }),
            layer(),
        );
        pipeline.register(
            content(Counter {
                paints: Rc::clone(&static_paints),
                color: Color::BLACK,
            }),
            static_layer,
        );
        let animated = pipeline.register(
            content(Counter {
                paints: Rc::clone(&animated_paints),
                color: Color::rgb8(255, 0, 0),
            }),
            animated_layer,
        );

        flush_paint(&mut pipeline);
        assert_eq!(static_paints.get(), 1);
        assert_eq!(animated_paints.get(), 1);

        // The animation marks its own boundary each frame, exactly as a driver would from `on_frame`.
        let scope = animated.scope();
        let _subscription = vsync.on_frame(move |_| scope.mark_needs_paint());

        for _ in 0..3 {
            vsync.tick(Duration::from_millis(16));
            flush_paint(&mut pipeline);
        }

        assert_eq!(
            animated_paints.get(),
            4,
            "the animated boundary repainted on each of the three frames"
        );
        assert_eq!(
            static_paints.get(),
            1,
            "the static boundary was painted once and reused throughout"
        );
    }
}
