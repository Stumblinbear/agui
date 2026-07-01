use std::cell::RefCell;
use std::rc::{Rc, Weak};

use slotmap::{Key, SlotMap};

use super::{FrameScheduler, PaintBoundaryId, PaintScope, RenderPipeline};

use crate::context::PaintCtx;
use crate::geometry::Offset;
use crate::paint::compositing::{LayerHandle, OffsetLayer};
use crate::paint::scene::SceneCapacity;
use crate::pipeline::{BoundaryContent, FramePhase};
use crate::render_object::{box_layout::RenderBox, node::MountedChild};

/// The repaint boundaries of one tree and their pending compositing-bit and repaint work, in two dirty lists
/// since compositing bits settle before paint.
pub(crate) struct PaintState {
    scheduler: Rc<FrameScheduler>,
    registry: RefCell<SlotMap<PaintBoundaryId, PaintBoundary>>,

    bits: RefCell<Vec<PaintBoundaryId>>,
    repaint: RefCell<Vec<PaintBoundaryId>>,

    deferred: Rc<RefCell<Vec<(PaintBoundaryId, PaintPhase)>>>,
}

impl PaintState {
    pub(super) fn new(scheduler: Rc<FrameScheduler>) -> Self {
        Self {
            scheduler,
            registry: RefCell::new(SlotMap::with_key()),
            bits: RefCell::new(Vec::new()),
            repaint: RefCell::new(Vec::new()),
            deferred: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Registers `content` as a repaint boundary nested under `enclosing`, painting into `layer`, returning the
    /// handle that owns and unregisters it. The new boundary is left unmarked: the caller either marks it (the
    /// root view) or paints it in the same pass it registers in (an inline boundary).
    pub(crate) fn register(
        self: &Rc<Self>,
        enclosing: PaintScope,
        content: PaintContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> PaintBoundaryHandle {
        let mut registry = self.registry.borrow_mut();

        let depth = registry.get(enclosing.0).map_or(0, |b| b.depth + 1);
        let id = registry.insert(PaintBoundary {
            depth,
            content,
            layer,
            paint_capacity: SceneCapacity::default(),
        });

        PaintBoundaryHandle {
            id,
            channel: Rc::downgrade(self),
        }
    }

    /// A deferred handle to `scope`'s repaint boundary.
    pub(crate) fn deferred_scope(&self, scope: PaintScope) -> DeferredPaintScope {
        DeferredPaintScope {
            id: scope.0,
            queue: Some(Rc::clone(&self.deferred)),
        }
    }

    /// Marks `id` for the work `phase` owes. Panics on a mark whose phase has run this frame, and ignores one
    /// for a boundary already gone. The composite phase owns no boundary, so a composite mark is dropped here.
    fn mark(&self, id: PaintBoundaryId, phase: PaintPhase) {
        self.scheduler.phase.get().assert_can_mark(match phase {
            PaintPhase::CompositingBits => FramePhase::CompositingBits,
            PaintPhase::Paint => FramePhase::Paint,
            PaintPhase::Composite => FramePhase::Composite,
        });

        if !self.registry.borrow().contains_key(id) {
            return;
        }

        match phase {
            PaintPhase::CompositingBits => {
                self.bits.borrow_mut().push(id);
                self.scheduler.notify();
            }

            PaintPhase::Paint => {
                self.repaint.borrow_mut().push(id);
                self.scheduler.notify();
            }

            PaintPhase::Composite => {}
        }
    }

    /// Schedules a recomposite of the subtree, with no boundary to repaint.
    pub(crate) fn mark_needs_composite(&self) {
        self.scheduler.notify();
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub(crate) fn mark_needs_paint(&self, scope: PaintScope) {
        self.mark(scope.0, PaintPhase::Paint);
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint.
    pub(crate) fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.mark(scope.0, PaintPhase::CompositingBits);
        self.mark(scope.0, PaintPhase::Paint);
    }
}

struct PaintBoundary {
    depth: usize,
    content: PaintContent,
    layer: LayerHandle<OffsetLayer>,
    /// The buffer lengths the last repaint recorded, sizing the next repaint's buffers.
    paint_capacity: SceneCapacity,
}

/// What a repaint boundary recomputes its compositing bits and repaints: the root view's render object, held
/// in a shared cell, or an inline boundary's child, reached by deferred handle. Both resolve to a
/// `&mut dyn RenderBox` the driver calls the two phase methods on, in their two separate passes.
#[derive(Clone)]
pub(crate) enum PaintContent {
    Root(BoundaryContent),
    Inline(MountedChild<dyn RenderBox>),
}

impl PaintContent {
    fn update_compositing_bits(&self) {
        match self {
            Self::Root(content) => {
                content.borrow_mut().dyn_update_compositing_bits();
            }
            Self::Inline(handle) => {
                handle.borrow_mut().update_compositing_bits();
            }
        }
    }

    fn paint(&self, ctx: &mut PaintCtx, offset: Offset) {
        match self {
            Self::Root(content) => content.borrow_mut().dyn_paint(ctx, offset),
            Self::Inline(handle) => handle.borrow_mut().paint(ctx, offset),
        }
    }
}

/// The work a repaint boundary owes, which are also the flush phases, in order: compositing bits settle
/// before paint.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PaintPhase {
    CompositingBits,
    Paint,
    Composite,
}

impl RenderPipeline {
    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.paint.mark_needs_compositing_bits_update(scope);
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.paint.mark_needs_paint(scope);
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting any boundary.
    pub fn mark_needs_composite(&self) {
        self.paint.mark_needs_composite();
    }

    /// Recomputes compositing bits, repaints, and recomposites the marked repaint boundaries.
    pub fn flush_paint(&self) {
        let deferred: Vec<(PaintBoundaryId, PaintPhase)> =
            self.paint.deferred.borrow_mut().drain(..).collect();

        for (id, phase) in deferred {
            match phase {
                PaintPhase::Paint => self.mark_needs_paint(PaintScope(id)),
                PaintPhase::CompositingBits => {
                    self.mark_needs_compositing_bits_update(PaintScope(id));
                }
                PaintPhase::Composite => self.mark_needs_composite(),
            }
        }

        self.flush_compositing_bits();
        self.flush_repaint();
    }

    fn flush_compositing_bits(&self) {
        let _phase = self.enter_phase(FramePhase::CompositingBits);

        let mut bits = self.paint.bits.borrow_mut();
        bits.sort_unstable();
        bits.dedup();
        {
            let registry = self.paint.registry.borrow();
            bits.sort_by_key(|&id| registry.get(id).map_or(0, |b| b.depth));
        }

        for &id in bits.iter() {
            let content = match self.paint.registry.borrow().get(id) {
                Some(boundary) => boundary.content.clone(),
                None => continue,
            };

            content.update_compositing_bits();
        }

        bits.clear();
    }

    fn flush_repaint(&self) {
        let _phase = self.enter_phase(FramePhase::Paint);

        let mut repaint = self.paint.repaint.borrow_mut();
        repaint.sort_unstable();
        repaint.dedup();
        {
            let registry = self.paint.registry.borrow();
            repaint.sort_by_key(|&id| registry.get(id).map_or(0, |b| b.depth));
        }

        for &id in repaint.iter() {
            let (content, layer, capacity) = match self.paint.registry.borrow().get(id) {
                Some(boundary) => (
                    boundary.content.clone(),
                    boundary.layer.clone(),
                    boundary.paint_capacity,
                ),
                None => continue,
            };

            layer.borrow_mut().clear();

            let recorded = PaintCtx::paint_with_capacity(
                &layer,
                capacity,
                &self.paint,
                PaintScope(id),
                |ctx| {
                    content.paint(ctx, Offset::ZERO);
                },
            );

            if let Some(boundary) = self.paint.registry.borrow_mut().get_mut(id) {
                boundary.paint_capacity = recorded;
            }
        }

        repaint.clear();
    }
}

/// The sole owner of a registered repaint boundary, held by the render object that established it. Dropping
/// it unregisters the boundary and discards its layer. Hand descendants the mark-only [`scope`](Self::scope),
/// and mark the boundary directly through this handle.
pub struct PaintBoundaryHandle {
    id: PaintBoundaryId,
    channel: Weak<PaintState>,
}

impl PaintBoundaryHandle {
    pub fn scope(&self) -> PaintScope {
        PaintScope(self.id)
    }

    /// Marks this boundary's compositing bits for recomputation before its next repaint, and the boundary
    /// for repaint.
    pub fn mark_needs_compositing_bits_update(&self) {
        if let Some(channel) = self.channel.upgrade() {
            channel.mark(self.id, PaintPhase::CompositingBits);
            channel.mark(self.id, PaintPhase::Paint);
        }
    }

    /// Marks this boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self) {
        if let Some(channel) = self.channel.upgrade() {
            channel.mark(self.id, PaintPhase::Paint);
        }
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting this boundary.
    pub fn mark_needs_composite(&self) {
        if let Some(channel) = self.channel.upgrade() {
            channel.mark_needs_composite();
        }
    }
}

impl Drop for PaintBoundaryHandle {
    fn drop(&mut self) {
        let Some(channel) = self.channel.upgrade() else {
            return;
        };

        // As for a layout boundary: the removed content's own drop can re-enter these borrows, so release
        // them before the content drops.
        let removed = {
            channel
                .deferred
                .borrow_mut()
                .retain(|(queued, _)| *queued != self.id);
            channel.registry.borrow_mut().remove(self.id)
        };
        drop(removed);
    }
}

/// A [`PaintScope`] paired with the route to mark it from outside a pipeline pass, such as a per-frame
/// animation callback. Each request is applied on the pipeline's next frame; a detached marker, or one whose
/// boundary is gone, marks nothing.
#[derive(Clone)]
pub struct DeferredPaintScope {
    id: PaintBoundaryId,
    #[allow(clippy::type_complexity)]
    queue: Option<Rc<RefCell<Vec<(PaintBoundaryId, PaintPhase)>>>>,
}

impl DeferredPaintScope {
    /// A marker detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self {
            id: PaintBoundaryId::null(),
            queue: None,
        }
    }

    /// Queues this boundary to be repainted on the pipeline's next frame.
    pub fn mark_needs_paint(&self) {
        self.push(PaintPhase::Paint);
    }

    /// Queues this boundary's compositing bits to be recomputed, and the boundary repainted.
    pub fn mark_needs_compositing_bits_update(&self) {
        self.push(PaintPhase::CompositingBits);
    }

    /// Queues a recomposite of the subtree, without repainting this boundary.
    pub fn mark_needs_composite(&self) {
        self.push(PaintPhase::Composite);
    }

    fn push(&self, phase: PaintPhase) {
        if let Some(queue) = &self.queue {
            queue.borrow_mut().push((self.id, phase));
        }
    }
}
