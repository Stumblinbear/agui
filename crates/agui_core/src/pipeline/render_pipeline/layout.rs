use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use slotmap::{Key, SlotMap};

use super::{FrameScheduler, LayoutBoundaryId, LayoutScope, PaintScope, RenderPipeline};

use crate::{
    build_queue::BuildQueue,
    context::UpdateCtx,
    pipeline::{
        FramePhase, RootElement,
        build_tree::Build,
        render_pipeline::{PaintState, SemanticsState},
    },
    provide::ProvideScope,
    scheduling::TaskScheduler,
    tree::NodeHandle,
};
use crate::{context::LayoutCtx, tree::Tree};

/// The content of one relayout boundary. It re-lays the boundary's subtree from the constraints it captured,
/// called with a [`LayoutCtx`] scoped to that boundary, and reports whether the layout it last produced can
/// be reused. The pipeline knows nothing of the layout protocol behind it: a box boundary re-lays its render
/// object under the box constraints it captured, and another protocol does the analogous thing for its own.
pub trait RelayoutContent {
    /// Re-lays the boundary's subtree from the constraints it captured.
    fn relayout(&mut self, ctx: &mut LayoutCtx);

    /// Whether the layout last produced stands for `constraints`, the layout protocol's constraint value,
    /// opaque here. A parent laying the boundary's subtree reuses it on `true`.
    fn reusable(&self, constraints: &dyn Any) -> bool;
}

/// A boxed [`RelayoutContent`], as a boundary registration takes it.
pub type RelayoutHook = Box<dyn RelayoutContent>;

/// The [`RelayoutContent`] of a bare re-lay closure, whose layout is never reusable.
pub struct RelayoutFn<F>(pub F);

impl<F: FnMut(&mut LayoutCtx)> RelayoutContent for RelayoutFn<F> {
    fn relayout(&mut self, ctx: &mut LayoutCtx) {
        (self.0)(ctx);
    }

    fn reusable(&self, _constraints: &dyn Any) -> bool {
        false
    }
}

/// The layout boundaries of one tree and their pending re-lays.
pub(crate) struct LayoutState {
    scheduler: Rc<FrameScheduler>,
    registry: RefCell<SlotMap<LayoutBoundaryId, LayoutBoundaryCell>>,

    // Ancestor-before-descendant by construction: build and layout both walk rootmost-first, and a children
    // walk's mark of its shared boundary is placed at the walk's floor (`mark_at`), ahead of the children's
    // deeper marks. So the flush drains in order with no sort, one entry per momentary borrow.
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

    /// Registers a relayout boundary, returning the handle that owns and unregisters it. The enclosing
    /// repaint boundary is recorded later, at paint, through
    /// [`set_paint_scope`](LayoutBoundaryHandle::set_paint_scope).
    pub(crate) fn register(self: &Rc<Self>, relayout: RelayoutHook) -> LayoutBoundaryHandle {
        let id = self.registry.borrow_mut().insert(LayoutBoundaryCell {
            queued: false,
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

        self.enqueue(id);
        self.scheduler.notify();
    }

    /// Marks `id` for re-layout at `floor`, a children walk's start in the dirty list, placing it ahead of
    /// the deeper marks the walk already made. Panics as [`mark`](Self::mark), and ignores a boundary already
    /// queued or already gone.
    pub(crate) fn mark_at(&self, id: LayoutBoundaryId, floor: usize) {
        self.scheduler
            .phase
            .get()
            .assert_can_mark(FramePhase::Layout);

        // A walk's own mark always sits at its floor, so this read answers "already marked" without touching
        // the registry.
        if self.dirty.borrow().get(floor) == Some(&id) {
            return;
        }

        if self.claim(id) {
            self.dirty.borrow_mut().insert(floor, id);
        }

        self.scheduler.notify();
    }

    /// Queues `id` for re-layout. A boundary already queued or already gone is left alone.
    fn enqueue(&self, id: LayoutBoundaryId) {
        if self.claim(id) {
            self.dirty.borrow_mut().push(id);
        }
    }

    /// Claims `id`'s single place in the dirty list: true when the caller should enter it, false when it is
    /// already queued or the boundary is gone.
    fn claim(&self, id: LayoutBoundaryId) -> bool {
        let mut registry = self.registry.borrow_mut();

        let Some(cell) = registry.get_mut(id) else {
            return false;
        };

        !std::mem::replace(&mut cell.queued, true)
    }

    /// The number of marks made so far, the floor a children walk beginning now hands to
    /// [`mark_at`](Self::mark_at).
    pub(crate) fn checkpoint(&self) -> usize {
        self.dirty.borrow().len()
    }
}

struct LayoutBoundaryCell {
    /// Whether a re-lay is already queued, so repeated marks put one entry in the dirty list.
    queued: bool,
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
    pub(crate) fn flush_layout(&self, host: &mut LayoutBuildHost) {
        for id in self.layout.deferred.take() {
            self.layout.enqueue(id);
        }

        // One momentary borrow per entry: a re-lay can re-enter build, and an element reconciled there marks
        // this list. Marks arrive ancestor-before-descendant (see `dirty`), so entries appended mid-flush
        // drain in order.
        let mut index = 0;

        loop {
            let entry = self.layout.dirty.borrow().get(index).copied();

            let Some(id) = entry else {
                break;
            };

            index += 1;

            let (relayout, paint) = {
                let mut registry = self.layout.registry.borrow_mut();

                // A boundary dropped since it was marked is absent, so skip it.
                let Some(cell) = registry.get_mut(id) else {
                    continue;
                };

                // A cleared mark means an ancestor's re-lay already laid this boundary's subtree this flush,
                // so its entry has nothing left to do.
                if !std::mem::replace(&mut cell.queued, false) {
                    continue;
                }

                (cell.relayout.take(), cell.paint)
            };

            let Some(mut relayout) = relayout else {
                continue;
            };

            let mut ctx = LayoutCtx::new(&self.layout, &self.paint, LayoutScope(id), host);
            relayout.relayout(&mut ctx);

            // Put the hook back, unless its own re-lay unregistered it.
            if let Some(cell) = self.layout.registry.borrow_mut().get_mut(id) {
                cell.relayout = Some(relayout);
            }

            // The boundary's painting is now stale, so repaint the boundary that encloses it.
            self.mark_needs_paint(paint);
        }

        self.layout.dirty.borrow_mut().clear();
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

    /// Whether the boundary's last layout stands for `constraints`, the layout protocol's constraint value;
    /// the caller reuses it in place of laying the subtree. A `false` obliges the caller to lay the subtree,
    /// and clears any pending re-lay mark, since that lay covers it and the flush then drops the boundary's
    /// queued entry.
    pub fn reuse_layout(&self, constraints: &dyn Any) -> bool {
        let Some(channel) = self.channel.upgrade() else {
            return false;
        };

        let mut registry = channel.registry.borrow_mut();

        let Some(cell) = registry.get_mut(self.id) else {
            return false;
        };

        if std::mem::replace(&mut cell.queued, false) {
            return false;
        }

        cell.relayout
            .as_deref()
            .is_some_and(|content| content.reusable(constraints))
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

/// The element-tree access a layout-time build needs, lent to the layout pass by the owner: the tree to
/// re-enter at a node, the dirty set, and the scope in force. A layout-time builder reaches it through
/// [`LayoutCtx::build_child`](crate::context::LayoutCtx::build_child) to build its child for the constraints it
/// was just handed. The scheduler comes separately, from the builder, which captured a deferred one at mount.
pub(crate) struct LayoutBuildHost<'a> {
    tree: &'a mut Tree<RootElement, Build>,

    queue: &'a mut BuildQueue,
    provide: ProvideScope,

    scheduler: &'a FrameScheduler,

    layout: &'a Rc<LayoutState>,
    paint: &'a Rc<PaintState>,
    semantics: &'a Rc<SemanticsState>,
}

impl<'a> LayoutBuildHost<'a> {
    pub(crate) fn new(
        tree: &'a mut Tree<RootElement, Build>,
        queue: &'a mut BuildQueue,
        provide: ProvideScope,
        pipeline: &'a RenderPipeline,
    ) -> Self {
        Self {
            tree,

            queue,
            provide,

            scheduler: &pipeline.scheduler,

            layout: pipeline.layout(),
            paint: pipeline.paint(),
            semantics: pipeline.semantics(),
        }
    }

    /// Hands `f` an [`UpdateCtx`] positioned at the element `handle` names, returning `f`'s result, or `None`
    /// if the element is gone.
    pub(crate) fn build<R>(
        &mut self,
        handle: NodeHandle,
        scheduler: &mut dyn TaskScheduler,
        f: impl FnOnce(&mut UpdateCtx) -> R,
    ) -> Option<R> {
        let provide = self.provide;
        let queue = &mut *self.queue;
        let layout = self.layout;
        let paint = self.paint;
        let semantics = self.semantics;

        let current_phase = self.scheduler.phase.get();

        // This method can only be called during the layout phase.
        debug_assert_eq!(current_phase, FramePhase::Layout);

        // We must re-enter build so elements can mark_needs_layout during this operation.
        self.scheduler.phase.set(FramePhase::Build);

        // `with_cursor`, not a dispatch op: the element is not reborrowed as `&mut`, so a render object's
        // in-flight layout borrow on it stands.
        let ret = self.tree.with_cursor(handle, |cursor| {
            let mut ctx =
                UpdateCtx::new(cursor, provide, queue, layout, paint, semantics, scheduler);

            f(&mut ctx)
        });

        self.scheduler.phase.set(current_phase);

        ret
    }
}
