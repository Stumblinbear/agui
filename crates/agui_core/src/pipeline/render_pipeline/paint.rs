use std::cell::RefCell;
use std::rc::{Rc, Weak};

use slotmap::{Key, SlotMap};

use super::{FrameScheduler, PaintBoundaryId, PaintScope, RenderPipeline};

use crate::pipeline::FramePhase;

/// A repaint hook: clears one boundary's layer and repaints its render object into it, scoped to the boundary
/// so nested boundaries registered during the repaint attribute correctly. It captures the layer, the paint
/// capacity, and the render object; the pipeline knows none of them.
pub type RepaintHook = Box<dyn FnMut(PaintScope)>;

/// A compositing-bits hook: recomputes one boundary's render object's compositing bits before its next
/// repaint, returning whether the compositing need changed, in which case the flush repaints the boundary.
pub type CompositingBitsHook = Box<dyn FnMut() -> bool>;

/// The repaint boundaries of one tree and their pending compositing-bit and repaint work, in two dirty lists
/// since compositing bits settle before paint.
pub struct PaintState {
    scheduler: Rc<FrameScheduler>,
    registry: RefCell<SlotMap<PaintBoundaryId, PaintBoundary>>,

    // Entries carry the depth captured at mark, so the flush sorts rootmost-first on inline keys with no
    // registry lookups; the cells' queued flags keep each boundary entered once.
    bits: RefCell<Vec<(usize, PaintBoundaryId)>>,
    repaint: RefCell<Vec<(usize, PaintBoundaryId)>>,

    deferred: Rc<PaintDeferred>,
}

/// Marks made out of a pipeline pass, applied at the start of the sub-flush that owns each: a compositing-bit
/// recompute before the bits settle, a repaint before the repaints run. A composite needs no boundary and is
/// covered by the frame's unconditional recomposite, so it is queued nowhere.
#[derive(Default)]
struct PaintDeferred {
    bits: RefCell<Vec<PaintBoundaryId>>,
    repaint: RefCell<Vec<PaintBoundaryId>>,
}

impl PaintState {
    pub(super) fn new(scheduler: Rc<FrameScheduler>) -> Self {
        Self {
            scheduler,
            registry: RefCell::new(SlotMap::with_key()),
            bits: RefCell::new(Vec::new()),
            repaint: RefCell::new(Vec::new()),
            deferred: Rc::new(PaintDeferred::default()),
        }
    }

    /// Registers a repaint boundary nested under `enclosing`, driven by `repaint` and `update_bits`, returning
    /// the handle that owns and unregisters it. The new boundary is left unmarked: the caller either marks it
    /// (the root view) or paints it in the same pass it registers in (an inline boundary).
    pub fn register(
        self: &Rc<Self>,
        enclosing: PaintScope,
        repaint: RepaintHook,
        update_bits: CompositingBitsHook,
    ) -> PaintBoundaryHandle {
        let mut registry = self.registry.borrow_mut();

        let depth = registry.get(enclosing.0).map_or(0, |b| b.depth + 1);
        let id = registry.insert(PaintBoundary {
            depth,
            bits_queued: false,
            repaint_queued: false,
            repaint: Some(repaint),
            update_bits: Some(update_bits),
        });

        PaintBoundaryHandle {
            id,
            channel: Rc::downgrade(self),
        }
    }

    /// A deferred handle to `scope`'s repaint boundary.
    pub fn deferred_scope(&self, scope: PaintScope) -> DeferredPaintScope {
        DeferredPaintScope {
            id: scope.0,
            queue: Some(Rc::clone(&self.deferred)),
        }
    }

    /// Queues `id` for a compositing-bit recompute at its depth, without requesting a frame. A boundary
    /// already queued or already gone is left alone.
    fn enqueue_bits(&self, id: PaintBoundaryId) {
        let mut registry = self.registry.borrow_mut();

        let Some(cell) = registry.get_mut(id) else {
            return;
        };

        if !std::mem::replace(&mut cell.bits_queued, true) {
            self.bits.borrow_mut().push((cell.depth, id));
        }
    }

    /// Queues `id` for a repaint at its depth, without requesting a frame. A boundary already queued or
    /// already gone is left alone.
    fn enqueue_repaint(&self, id: PaintBoundaryId) {
        let mut registry = self.registry.borrow_mut();

        let Some(cell) = registry.get_mut(id) else {
            return;
        };

        if !std::mem::replace(&mut cell.repaint_queued, true) {
            self.repaint.borrow_mut().push((cell.depth, id));
        }
    }

    /// Whether repaint work is pending: a boundary's bits or repaint marked, in a list or still deferred.
    pub(crate) fn has_pending(&self) -> bool {
        !self.bits.borrow().is_empty()
            || !self.repaint.borrow().is_empty()
            || !self.deferred.bits.borrow().is_empty()
            || !self.deferred.repaint.borrow().is_empty()
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and requests a frame.
    ///
    /// # Panics
    /// Panics if the compositing-bits phase has already run this frame.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.scheduler
            .phase
            .get()
            .assert_can_mark(FramePhase::CompositingBits);

        self.enqueue_bits(scope.0);
        self.scheduler.notify();
    }

    /// Marks `scope`'s boundary to be repainted on the next frame, and requests one. Permitted during the
    /// compositing-bits phase, so a bits recompute that changes the need can mark the repaint it now owes.
    ///
    /// # Panics
    /// Panics if the paint phase has already run this frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.scheduler
            .phase
            .get()
            .assert_can_mark(FramePhase::Paint);

        self.enqueue_repaint(scope.0);
        self.scheduler.notify();
    }

    /// Requests a frame to recomposite the subtree. Compositing owns no boundary and a frame recomposites
    /// unconditionally, so this only ensures a frame runs.
    ///
    /// # Panics
    /// Panics if the composite phase has already run this frame.
    pub fn mark_needs_composite(&self) {
        self.scheduler
            .phase
            .get()
            .assert_can_mark(FramePhase::Composite);

        self.scheduler.notify();
    }
}

struct PaintBoundary {
    depth: usize,

    bits_queued: bool,
    update_bits: Option<CompositingBitsHook>,

    repaint_queued: bool,
    repaint: Option<RepaintHook>,
}

impl RenderPipeline {
    /// Marks `scope`'s compositing bits for recomputation before its next repaint. The boundary repaints when
    /// the recomputation finds its compositing need changed.
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
        self.flush_compositing_bits();
        self.flush_repaint();
    }

    fn flush_compositing_bits(&self) {
        let _phase = self.enter_phase(FramePhase::CompositingBits);

        // Fold in the compositing-bit marks made out of pass this frame, right before the bits settle.
        for id in std::mem::take(&mut *self.paint.deferred.bits.borrow_mut()) {
            self.paint.enqueue_bits(id);
        }

        let mut bits = self.paint.bits.borrow_mut();
        bits.sort_unstable_by_key(|&(depth, _)| depth);

        for &(_, id) in bits.iter() {
            // Take the hook out so it can re-enter the registry, and put it back after.
            let hook = match self.paint.registry.borrow_mut().get_mut(id) {
                Some(boundary) => {
                    boundary.bits_queued = false;
                    boundary.update_bits.take()
                }

                None => continue,
            };

            let Some(mut hook) = hook else { continue };

            let changed = hook();

            if let Some(boundary) = self.paint.registry.borrow_mut().get_mut(id) {
                boundary.update_bits = Some(hook);
            }

            if changed {
                self.paint.mark_needs_paint(PaintScope(id));
            }
        }

        bits.clear();
    }

    fn flush_repaint(&self) {
        let _phase = self.enter_phase(FramePhase::Paint);

        // Fold in the repaint marks made out of pass this frame, and any the bits flush just added.
        for id in std::mem::take(&mut *self.paint.deferred.repaint.borrow_mut()) {
            self.paint.enqueue_repaint(id);
        }

        let mut repaint = self.paint.repaint.borrow_mut();
        repaint.sort_unstable_by_key(|&(depth, _)| depth);

        for &(_, id) in repaint.iter() {
            // Take the hook out so it can re-enter the registry during its own repaint, and put it back after.
            let hook = match self.paint.registry.borrow_mut().get_mut(id) {
                Some(boundary) => {
                    boundary.repaint_queued = false;
                    boundary.repaint.take()
                }

                None => continue,
            };

            let Some(mut hook) = hook else { continue };

            hook(PaintScope(id));

            if let Some(boundary) = self.paint.registry.borrow_mut().get_mut(id) {
                boundary.repaint = Some(hook);
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

    /// Marks this boundary's compositing bits for recomputation before its next repaint. The boundary
    /// repaints when the recomputation finds its compositing need changed.
    pub fn mark_needs_compositing_bits_update(&self) {
        if let Some(channel) = self.channel.upgrade() {
            channel.mark_needs_compositing_bits_update(PaintScope(self.id));
        }
    }

    /// Records the repaint boundary now enclosing this one, refreshing the depth the flush drains
    /// rootmost-first by. The render object that established this boundary calls it during paint, where the
    /// enclosing boundary is known, so a boundary whose nesting changed drains at its current depth rather
    /// than the one it was registered under.
    pub fn set_enclosing(&self, enclosing: PaintScope) {
        let Some(channel) = self.channel.upgrade() else {
            return;
        };

        let mut registry = channel.registry.borrow_mut();

        let depth = registry.get(enclosing.0).map_or(0, |b| b.depth + 1);

        if let Some(cell) = registry.get_mut(self.id) {
            cell.depth = depth;
        }
    }

    /// Marks this boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self) {
        if let Some(channel) = self.channel.upgrade() {
            channel.mark_needs_paint(PaintScope(self.id));
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
        let removed = { channel.registry.borrow_mut().remove(self.id) };

        drop(removed);
    }
}

/// A [`PaintScope`] paired with the route to mark it from outside a pipeline pass, such as a per-frame
/// animation callback. Each request is applied on the pipeline's next frame; a detached marker, or one whose
/// boundary is gone, marks nothing.
#[derive(Clone)]
pub struct DeferredPaintScope {
    id: PaintBoundaryId,
    queue: Option<Rc<PaintDeferred>>,
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
        if let Some(queue) = &self.queue {
            queue.repaint.borrow_mut().push(self.id);
        }
    }

    /// Queues this boundary's compositing bits to be recomputed on the pipeline's next frame; the boundary
    /// repaints when the recomputation finds its compositing need changed.
    pub fn mark_needs_compositing_bits_update(&self) {
        if let Some(queue) = &self.queue {
            queue.bits.borrow_mut().push(self.id);
        }
    }

    pub fn mark_needs_composite(&self) {}
}
