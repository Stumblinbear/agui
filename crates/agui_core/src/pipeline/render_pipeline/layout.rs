use std::cell::RefCell;
use std::rc::{Rc, Weak};

use slotmap::{Key, SlotMap};

use super::{FrameScheduler, LayoutBoundaryId, LayoutScope, PaintScope, RenderPipeline};

use crate::context::LayoutCtx;
use crate::pipeline::{FramePhase, LayoutBuildHost};

/// A relayout hook: re-lays one boundary's subtree from the constraints it captured. The pipeline calls it
/// with a [`LayoutCtx`] scoped to that boundary, knowing nothing of the layout protocol behind it. A box
/// boundary re-lays its render object under the box constraints it captured; another protocol does the
/// analogous thing for its own.
pub type RelayoutHook = Box<dyn FnMut(&mut LayoutCtx)>;

/// The layout boundaries of one tree and their pending re-lays. `registry` and `dirty` are separate cells so a
/// flush holds the marks while the relay re-enters the registry to register nested boundaries.
pub(crate) struct LayoutState {
    scheduler: Rc<FrameScheduler>,
    registry: RefCell<SlotMap<LayoutBoundaryId, LayoutBoundaryCell>>,

    dirty: RefCell<Vec<LayoutBoundaryId>>,
    deferred: Rc<RefCell<Vec<LayoutBoundaryId>>>,
}

impl LayoutState {
    pub(super) fn new(scheduler: Rc<FrameScheduler>) -> Self {
        Self {
            scheduler,
            registry: RefCell::new(SlotMap::with_key()),
            dirty: RefCell::new(Vec::new()),
            deferred: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Registers `boundary` as a relayout boundary nested under `enclosing`, returning the handle that owns and
    /// unregisters it. The enclosing repaint boundary is recorded later, at paint, through
    /// [`set_paint_scope`](LayoutBoundaryHandle::set_paint_scope).
    pub(crate) fn register(
        self: &Rc<Self>,
        enclosing: LayoutScope,
        relayout: RelayoutHook,
    ) -> LayoutBoundaryHandle {
        let mut registry = self.registry.borrow_mut();

        let depth = registry.get(enclosing.0).map_or(0, |b| b.depth + 1);
        let id = registry.insert(LayoutBoundaryCell {
            depth,
            relayout: Some(relayout),
            paint: PaintScope::detached(),
        });

        LayoutBoundaryHandle {
            id,
            channel: Rc::downgrade(self),
            paint: PaintScope::detached(),
        }
    }

    /// A deferred handle to `scope`'s boundary, for marking it from a callback that holds no pipeline, such as
    /// a reconcile or a per-frame animation.
    pub(crate) fn deferred_scope(&self, scope: LayoutScope) -> DeferredLayoutScope {
        DeferredLayoutScope {
            id: scope.0,
            queue: Some(Rc::clone(&self.deferred)),
        }
    }

    /// Marks `id` for re-layout. Panics on a mark made once the layout phase has run this frame, and ignores
    /// one for a boundary already gone.
    pub(crate) fn mark(&self, id: LayoutBoundaryId) {
        self.scheduler
            .phase
            .get()
            .assert_can_mark(FramePhase::Layout);

        if !self.registry.borrow().contains_key(id) {
            return;
        }

        self.dirty.borrow_mut().push(id);
        self.scheduler.notify();
    }
}

struct LayoutBoundaryCell {
    /// Depth in the boundary nesting, so a drain re-enters rootmost-first.
    depth: usize,
    /// The relayout hook, owned here. Taken out for the duration of its own re-lay, so the re-lay can re-enter
    /// the pipeline to register nested boundaries, and put back after.
    relayout: Option<RelayoutHook>,
    /// The repaint boundary enclosing this one, marked when this boundary re-lays so the re-laid subtree
    /// repaints.
    paint: PaintScope,
}

impl RenderPipeline {
    /// Re-lays every marked relayout boundary from the constraints it last took, rootmost-first, leaving the
    /// rest untouched.
    pub(crate) fn flush_layout(&self, host: &RefCell<LayoutBuildHost>) {
        // Hold the marks across the pass. Marking layout during the layout phase is rejected, so nothing else
        // touches this list while a relay re-enters the registry (a separate cell) to register nested boundaries.
        let mut dirty = self.layout.dirty.borrow_mut();
        dirty.extend(self.layout.deferred.borrow_mut().drain(..));
        dirty.sort_unstable();
        dirty.dedup();
        {
            let registry = self.layout.registry.borrow();
            dirty.sort_by_key(|&id| registry.get(id).map_or(0, |b| b.depth));
        }

        for &id in dirty.iter() {
            let (relayout, paint) = {
                let mut registry = self.layout.registry.borrow_mut();

                // A boundary dropped since it was marked is absent, so skip it.
                let Some(cell) = registry.get_mut(id) else {
                    continue;
                };

                (cell.relayout.take(), cell.paint)
            };

            let Some(mut relayout) = relayout else {
                continue;
            };

            let mut ctx = LayoutCtx::new(&self.layout, &self.paint, LayoutScope(id), host);
            relayout(&mut ctx);

            // Put the hook back, unless its own re-lay unregistered it.
            if let Some(cell) = self.layout.registry.borrow_mut().get_mut(id) {
                cell.relayout = Some(relayout);
            }

            // The boundary's painting is now stale, so repaint the boundary that encloses it.
            self.mark_needs_paint(paint);
        }

        dirty.clear();
    }
}

/// The sole owner of a registered relayout boundary, held by the render object that established it. Dropping
/// it unregisters the boundary. Hand descendants the [`scope`](Self::scope) to mark, and request a re-layout
/// of the boundary itself with [`mark_needs_layout`](Self::mark_needs_layout).
pub struct LayoutBoundaryHandle {
    id: LayoutBoundaryId,
    channel: Weak<LayoutState>,
    /// The enclosing repaint boundary last written to the cell, so a repeated [`set_paint_scope`] with the
    /// same value (the steady state every paint) skips the upgrade and write.
    paint: PaintScope,
}

impl LayoutBoundaryHandle {
    pub fn scope(&self) -> LayoutScope {
        LayoutScope(self.id)
    }

    /// Swaps the boundary's content while keeping its id, so the constraints it re-lays from stay current
    /// without re-registering (which would change the scope descendants captured). The render object replaces
    /// it on each layout that finds it still a boundary.
    pub fn replace(&mut self, relayout: RelayoutHook) {
        if let Some(channel) = self.channel.upgrade()
            && let Some(cell) = channel.registry.borrow_mut().get_mut(self.id)
        {
            cell.relayout = Some(relayout);
        }
    }

    /// Records the repaint boundary enclosing this one, so a re-lay can mark it for repaint. The render object
    /// that established this boundary calls it during paint, where the enclosing repaint boundary is known.
    pub fn set_paint_scope(&mut self, paint: PaintScope) {
        if self.paint == paint {
            return;
        }

        self.paint = paint;

        if let Some(channel) = self.channel.upgrade()
            && let Some(cell) = channel.registry.borrow_mut().get_mut(self.id)
        {
            cell.paint = paint;
        }
    }

    /// Requests that this boundary be re-laid before the next frame.
    pub fn mark_needs_layout(&self) {
        if let Some(channel) = self.channel.upgrade() {
            channel.mark(self.id);
        }
    }
}

impl Drop for LayoutBoundaryHandle {
    fn drop(&mut self) {
        let Some(channel) = self.channel.upgrade() else {
            return;
        };

        // A pending mark for this id is left in the dirty list: the next flush drops it when it finds no cell.
        let removed = channel.registry.borrow_mut().remove(self.id);

        // Drop the removed cell only after the borrow is released: it may own a nested boundary (a child
        // render object's), whose own drop re-borrows the registry to unregister.
        drop(removed);
    }
}

/// A [`LayoutScope`] paired with the route to mark it from outside a pipeline pass, such as a reconcile that
/// changes a layout property. The request is applied on the pipeline's next frame; a detached marker, or one
/// whose boundary is gone, marks nothing.
#[derive(Clone)]
pub struct DeferredLayoutScope {
    id: LayoutBoundaryId,
    queue: Option<Rc<RefCell<Vec<LayoutBoundaryId>>>>,
}

impl DeferredLayoutScope {
    /// A marker detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self {
            id: LayoutBoundaryId::null(),
            queue: None,
        }
    }

    /// Queues this boundary to be re-laid on the pipeline's next frame.
    pub fn mark_needs_layout(&self) {
        if let Some(queue) = &self.queue {
            queue.borrow_mut().push(self.id);
        }
    }
}
