// The intrusive-collections adapter macro emits a manual `Clone` on a zero-sized `Copy` adapter.
#![allow(clippy::expl_impl_clone_on_copy)]

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use intrusive_collections::{LinkedList, LinkedListLink, UnsafeRef, intrusive_adapter};

use crate::{
    context::PaintCtx,
    geometry::Offset,
    paint::compositing::{LayerHandle, OffsetLayer},
    paint::scene::SceneCapacity,
    pipeline::BoundaryContent,
    render_object::{RenderObject, box_layout::RenderBox},
};

/// Owns the repaint boundaries of one subtree and repaints the ones that have changed.
///
/// The subtree's root paints into the layer composited for presentation; every boundary nested inside
/// paints into its own retained layer and can be repainted on its own, leaving every other boundary's
/// layer untouched. Mark the root through the [`PaintBoundaryHandle`] returned when the pipeline is
/// built, and an inner boundary through the one handed back when it is registered.
pub struct PaintPipeline {
    pending: Rc<RefCell<PaintPipelineState>>,
}

fn noop() {}

impl Default for PaintPipeline {
    fn default() -> Self {
        Self {
            pending: Rc::new(RefCell::new(PaintPipelineState {
                needs_paint: LinkedList::new(PaintLinkAdapter::new()),
                needs_compositing: LinkedList::new(CompositingLinkAdapter::new()),
                needs_composite: false,
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
        layer: LayerHandle<OffsetLayer>,
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
        layer: LayerHandle<OffsetLayer>,
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

        let cell = Rc::new(PaintCell {
            paint_link: LinkedListLink::new(),
            compositing_link: LinkedListLink::new(),
            pending: Rc::downgrade(&self.pending),
            content,
            layer,
            paint_capacity: Cell::new(SceneCapacity::default()),
            needs_paint: Cell::new(false),
            needs_compositing: Cell::new(false),
        });

        // A fresh boundary has never been painted and its bits have never been computed.
        {
            let mut pending = self.pending.borrow_mut();
            pending.link_paint(&cell);
            pending.link_compositing(&cell);
        }

        tracing::debug!(boundary = ?Rc::as_ptr(&cell), "registered repaint boundary");

        PaintBoundaryHandle { cell }
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

        // Unregistration is the handle's `Drop`; the explicit drop keeps that load-bearing step visible.
        drop(handle);
    }

    /// Recomputes the compositing bits of every marked inner boundary, settling them before any repaint
    /// reads them.
    ///
    /// # Panics
    ///
    /// Panics if called while updating compositing bits or painting is in progress.
    pub fn flush_compositing_bits(&mut self) {
        {
            let pending = self.pending.borrow();

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
        }

        self.pending.borrow_mut().phase = PaintPipelinePhase::UpdateCompositingBits;

        loop {
            let cell = {
                let mut pending = self.pending.borrow_mut();

                let Some(unlinked) = pending.needs_compositing.pop_front() else {
                    break;
                };

                unlinked.needs_compositing.set(false);
                recover_owner(unlinked)
            };

            let mut content = Rc::clone(&cell.content);
            content.update_compositing_bits();
        }

        self.pending.borrow_mut().phase = PaintPipelinePhase::Idle;
    }

    /// Repaints every marked inner boundary into its own layer, leaving the rest as they are.
    ///
    /// # Panics
    ///
    /// Panics if called while updating compositing bits or painting is in progress.
    pub fn flush_paint(&mut self) {
        {
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

            // This flush is part of the frame that composites, so any standalone composite request is
            // about to be satisfied; a later mark schedules the next frame afresh.
            pending.needs_composite = false;

            if pending.needs_paint.is_empty() {
                return;
            }
        }

        self.pending.borrow_mut().phase = PaintPipelinePhase::Paint;

        loop {
            let cell = {
                let mut pending = self.pending.borrow_mut();

                let Some(unlinked) = pending.needs_paint.pop_front() else {
                    break;
                };

                unlinked.needs_paint.set(false);
                recover_owner(unlinked)
            };

            tracing::debug!(boundary = ?Rc::as_ptr(&cell), "repainting boundary");

            let layer = &cell.layer;

            layer.borrow_mut().clear();

            let mut content = Rc::clone(&cell.content);
            let recorded = PaintCtx::paint_with_capacity(layer, cell.paint_capacity.get(), |ctx| {
                content.paint(ctx, Offset::ZERO);
            });
            cell.paint_capacity.set(recorded);
        }

        self.pending.borrow_mut().phase = PaintPipelinePhase::Idle;
    }
}

impl Drop for PaintPipeline {
    fn drop(&mut self) {
        // The channels hold non-owning references into cells the registry is about to free; empty them
        // first so no link outlives its cell.
        let mut pending = self.pending.borrow_mut();
        pending.needs_paint.fast_clear();
        pending.needs_compositing.fast_clear();
    }
}

enum PaintPipelinePhase {
    Idle,
    UpdateCompositingBits,
    Paint,
}

/// A registered repaint boundary, owned by the pipeline registry for as long as it is registered and
/// linked into the paint or compositing channel while it awaits work on that channel.
struct PaintCell {
    paint_link: LinkedListLink,
    compositing_link: LinkedListLink,

    /// The pipeline state this cell's marks are linked into.
    pending: Weak<RefCell<PaintPipelineState>>,

    content: BoundaryContent,
    layer: LayerHandle<OffsetLayer>,

    /// The buffer lengths the last repaint recorded, sizing the next repaint's buffers.
    paint_capacity: Cell<SceneCapacity>,

    /// Whether this cell is currently in the paint channel, guarding a double-mark from linking it twice.
    needs_paint: Cell<bool>,

    /// Whether this cell is currently in the compositing-bits channel, guarding a double-mark from linking
    /// it twice.
    needs_compositing: Cell<bool>,
}

intrusive_adapter!(PaintLinkAdapter = UnsafeRef<PaintCell>: PaintCell { paint_link => LinkedListLink });
intrusive_adapter!(CompositingLinkAdapter = UnsafeRef<PaintCell>: PaintCell { compositing_link => LinkedListLink });

/// Recovers a counted owning `Rc` from a non-owning ref popped off a channel.
fn recover_owner(popped: UnsafeRef<PaintCell>) -> Rc<PaintCell> {
    let ptr = UnsafeRef::into_raw(popped);

    // SAFETY: the ref was created with `UnsafeRef::from_raw(Rc::as_ptr(&cell))`, so `ptr` carries the
    // whole-allocation provenance of a live `Rc<PaintCell>` whose owner outlives this call. Bumping the
    // strong count before reconstructing balances the `Rc` this produces against that still-live owner.
    unsafe {
        Rc::increment_strong_count(ptr);
        Rc::from_raw(ptr)
    }
}

/// The boundaries awaiting repaint or a bit recompute, and the hook that schedules a frame.
struct PaintPipelineState {
    needs_compositing: LinkedList<CompositingLinkAdapter>,
    needs_paint: LinkedList<PaintLinkAdapter>,

    /// Whether a layer's placement changed and the subtree must recomposite, with no boundary to
    /// repaint. Set out of band by an animation that moves a retained layer; cleared when the frame
    /// composites.
    needs_composite: bool,

    notify: Box<dyn Fn()>,

    phase: PaintPipelinePhase,
}

impl PaintPipelineState {
    /// Returns `true` if nothing is awaiting repaint, compositing update, or recomposite.
    fn is_clean(&self) -> bool {
        self.needs_paint.is_empty() && self.needs_compositing.is_empty() && !self.needs_composite
    }

    fn link_paint(&mut self, cell: &Rc<PaintCell>) {
        if cell.needs_paint.get() {
            return;
        }

        cell.needs_paint.set(true);
        // SAFETY: the cell is owned by an `Rc` in the registry for as long as it is registered, and it
        // is unlinked before that `Rc` is dropped in `unregister`; the per-channel flag guards it
        // against being linked into this channel more than once.
        self.needs_paint
            .push_back(unsafe { UnsafeRef::from_raw(Rc::as_ptr(cell)) });
    }

    fn unlink_paint(&mut self, cell: &Rc<PaintCell>) {
        if !cell.needs_paint.get() {
            return;
        }

        // SAFETY: the cell is linked into this channel and stays live behind its `Rc` until removed
        // here, so the pointer the cursor recovers is valid.
        let mut cursor = unsafe { self.needs_paint.cursor_mut_from_ptr(Rc::as_ptr(cell)) };
        cursor.remove();
        cell.needs_paint.set(false);
    }

    fn link_compositing(&mut self, cell: &Rc<PaintCell>) {
        if cell.needs_compositing.get() {
            return;
        }

        cell.needs_compositing.set(true);
        // SAFETY: the cell is owned by an `Rc` in the registry for as long as it is registered, and it
        // is unlinked before that `Rc` is dropped in `unregister`; the per-channel flag guards it
        // against being linked into this channel more than once.
        self.needs_compositing
            .push_back(unsafe { UnsafeRef::from_raw(Rc::as_ptr(cell)) });
    }

    fn unlink_compositing(&mut self, cell: &Rc<PaintCell>) {
        if !cell.needs_compositing.get() {
            return;
        }

        // SAFETY: the cell is linked into this channel and stays live behind its `Rc` until removed
        // here, so the pointer the cursor recovers is valid.
        let mut cursor = unsafe { self.needs_compositing.cursor_mut_from_ptr(Rc::as_ptr(cell)) };
        cursor.remove();
        cell.needs_compositing.set(false);
    }

    fn mark_needs_paint(&mut self, cell: &Rc<PaintCell>) {
        match self.phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!("cannot mark a render object for paint while updating compositing bits");
            }

            PaintPipelinePhase::Paint => {
                panic!("cannot mark a render object for paint while painting");
            }
        }

        tracing::trace!(boundary = ?Rc::as_ptr(cell), "marked boundary for repaint");

        let was_clean = self.is_clean();

        self.link_paint(cell);

        if was_clean {
            (self.notify)();
        }
    }

    fn mark_needs_compositing_bits_update(&mut self, cell: &Rc<PaintCell>) {
        match self.phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!(
                    "cannot mark a render object for compositing bits update while updating compositing bits"
                );
            }

            PaintPipelinePhase::Paint => {
                panic!(
                    "cannot mark a render object for compositing bits update while painting is in progress"
                );
            }
        }

        tracing::trace!(boundary = ?Rc::as_ptr(cell), "marked boundary for compositing bits update");

        let was_clean = self.is_clean();

        self.link_compositing(cell);
        // A changed compositing bit changes how the boundary paints, so it must also repaint.
        self.link_paint(cell);

        if was_clean {
            (self.notify)();
        }
    }

    fn mark_needs_composite(&mut self) {
        match self.phase {
            PaintPipelinePhase::Idle => {}

            PaintPipelinePhase::UpdateCompositingBits => {
                panic!("cannot mark a recomposite while updating compositing bits");
            }

            PaintPipelinePhase::Paint => {
                panic!("cannot mark a recomposite while painting is in progress");
            }
        }

        let was_clean = self.is_clean();

        self.needs_composite = true;

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
pub struct PaintScope(Weak<PaintCell>);

impl PaintScope {
    /// A scope detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self(Weak::new())
    }

    /// Whether marks through this scope reach nothing, because the scope was created detached or its
    /// boundary has since been freed.
    pub fn is_detached(&self) -> bool {
        Weak::strong_count(&self.0) == 0
    }

    /// Marks the boundary to be repainted on the next frame. If this is the first mark on an otherwise
    /// clean pipeline, the schedule hook fires so the driver schedules a frame.
    pub fn mark_needs_paint(&self) {
        let Some(cell) = self.0.upgrade() else {
            return;
        };

        let Some(pending) = cell.pending.upgrade() else {
            return;
        };

        pending.borrow_mut().mark_needs_paint(&cell);
    }

    /// Marks the boundary's compositing bits for recomputation before its next repaint, and the
    /// boundary for repaint. Only an inner boundary tracks its bits this way.
    pub fn mark_needs_compositing_bits_update(&self) {
        let Some(cell) = self.0.upgrade() else {
            return;
        };

        let Some(pending) = cell.pending.upgrade() else {
            return;
        };

        pending
            .borrow_mut()
            .mark_needs_compositing_bits_update(&cell);
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting any boundary. Use
    /// it when only a retained layer's placement changed, as a per-frame animation of a transform or
    /// offset does.
    pub fn mark_needs_composite(&self) {
        let Some(cell) = self.0.upgrade() else {
            return;
        };

        let Some(pending) = cell.pending.upgrade() else {
            return;
        };

        pending.borrow_mut().mark_needs_composite();
    }
}

/// A registered boundary, returned to the render object that registered it. It owns the boundary's
/// place in the pipeline and is the only thing [`unregister`](PaintPipeline::unregister) accepts; it
/// hands out mark-only [`PaintScope`]s for the subtree. Keeping removal here, off the scope, stops a
/// descendant that was handed a scope to mark with from unregistering the boundary it lives under.
pub struct PaintBoundaryHandle {
    cell: Rc<PaintCell>,
}

impl PaintBoundaryHandle {
    /// A mark-only handle to this boundary, for descendants to repaint into it.
    pub fn scope(&self) -> PaintScope {
        PaintScope(Rc::downgrade(&self.cell))
    }

    /// Marks this boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self) {
        self.scope().mark_needs_paint();
    }

    /// Marks this boundary's compositing bits for recomputation before its next repaint.
    pub fn mark_needs_compositing_bits_update(&self) {
        self.scope().mark_needs_compositing_bits_update();
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting this boundary.
    pub fn mark_needs_composite(&self) {
        self.scope().mark_needs_composite();
    }
}

impl Drop for PaintBoundaryHandle {
    fn drop(&mut self) {
        // The channels hold non-owning refs into this cell; unlink it before its `Rc` frees it.
        let Some(pending) = self.cell.pending.upgrade() else {
            return;
        };

        let mut pending = pending.borrow_mut();
        pending.unlink_paint(&self.cell);
        pending.unlink_compositing(&self.cell);
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
        children: Vec<LayerHandle<OffsetLayer>>,
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
                ctx.add_layer(child.clone(), offset);
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

    fn layer() -> LayerHandle<OffsetLayer> {
        LayerHandle::new(OffsetLayer::new())
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

    /// A scope is cloned onto every node that can mark its boundary, so it stays one pointer wide.
    #[test]
    fn a_paint_scope_is_one_pointer() {
        assert_eq!(
            std::mem::size_of::<PaintScope>(),
            std::mem::size_of::<usize>()
        );
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
                color: Color::from_rgb8(255, 0, 0),
            }),
            child_layer,
        );

        flush_paint(&mut pipeline);
        let first = Compositor::compose(&root_layer).rasterize();
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
        let second = Compositor::compose(&root_layer).rasterize();
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

        boundary.mark_needs_compositing_bits_update();
        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 3, "the boundary repainted");
        assert_eq!(bits.get(), 2, "a compositing-bits mark recomputes bits");
    }

    #[test]
    fn a_composite_mark_schedules_a_frame_without_repainting() {
        let frames = Rc::new(Cell::new(0));
        let scheduled = Rc::clone(&frames);
        let paints = Rc::new(Cell::new(0));

        let (mut pipeline, boundary) = PaintPipeline::new(
            content(Counter {
                paints: Rc::clone(&paints),
                color: Color::BLACK,
            }),
            layer(),
        );
        pipeline.on_needs_paint(Box::new(move || scheduled.set(scheduled.get() + 1)));

        // Registration left the boundary dirty; the first frame clears it without involving the hook.
        flush_paint(&mut pipeline);
        assert_eq!(paints.get(), 1);
        assert_eq!(frames.get(), 0, "registration alone schedules no frame");

        boundary.mark_needs_composite();
        boundary.mark_needs_composite();
        assert_eq!(
            frames.get(),
            1,
            "only the clean-to-dirty edge schedules a frame"
        );

        flush_paint(&mut pipeline);
        assert_eq!(
            paints.get(),
            1,
            "a composite mark recomposites without repainting the boundary"
        );

        boundary.mark_needs_composite();
        assert_eq!(
            frames.get(),
            2,
            "a mark after the flush schedules another frame"
        );
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
        // Hold the static boundary's handle: it owns the registration, so dropping it would unregister.
        let _static = pipeline.register(
            content(Counter {
                paints: Rc::clone(&static_paints),
                color: Color::BLACK,
            }),
            static_layer,
        );
        let animated = pipeline.register(
            content(Counter {
                paints: Rc::clone(&animated_paints),
                color: Color::from_rgb8(255, 0, 0),
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
