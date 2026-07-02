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

    /// Marks `id` for the work `phase` owes. Panics on a mark whose phase has run this frame, and ignores one
    /// for a boundary already gone. The composite phase owns no boundary, so a composite mark is dropped here.
    fn mark(&self, id: PaintBoundaryId, phase: PaintPhase) {
        self.scheduler.phase.get().assert_can_mark(match phase {
            PaintPhase::CompositingBits => FramePhase::CompositingBits,
            PaintPhase::Paint => FramePhase::Paint,
            PaintPhase::Composite => FramePhase::Composite,
        });

        {
            let mut registry = self.registry.borrow_mut();

            let Some(cell) = registry.get_mut(id) else {
                return;
            };

            let depth = cell.depth;
            let (flag, list) = match phase {
                PaintPhase::CompositingBits => (&mut cell.bits_queued, &self.bits),
                PaintPhase::Paint => (&mut cell.repaint_queued, &self.repaint),
                PaintPhase::Composite => return,
            };

            if !std::mem::replace(flag, true) {
                list.borrow_mut().push((depth, id));
            }
        }

        self.scheduler.notify();
    }

    /// Schedules a recomposite of the subtree, with no boundary to repaint.
    pub fn mark_needs_composite(&self) {
        self.scheduler.notify();
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.mark(scope.0, PaintPhase::Paint);
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint. The boundary repaints when
    /// the recomputation finds its compositing need changed.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.mark(scope.0, PaintPhase::CompositingBits);
    }
}

struct PaintBoundary {
    depth: usize,

    bits_queued: bool,
    update_bits: Option<CompositingBitsHook>,

    repaint_queued: bool,
    repaint: Option<RepaintHook>,
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
            channel.mark(self.id, PaintPhase::CompositingBits);
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

    /// Queues this boundary's compositing bits to be recomputed on the pipeline's next frame; the boundary
    /// repaints when the recomputation finds its compositing need changed.
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
