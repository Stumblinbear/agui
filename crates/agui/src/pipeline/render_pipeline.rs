//! The relayout and repaint boundaries of one widget tree, merged into a single pipeline the
//! [`PipelineOwner`](super::PipelineOwner) holds and drives. Each boundary is opaque, registered by id,
//! marked stale through its handle, and drained shallowest-first for the driver to recompute.
//!
//! A render object registers a boundary the first time it qualifies (a relayout boundary under tight
//! constraints, a repaint boundary at paint), keeps the handle it gets back, and marks the boundary stale
//! through it or through a [`LayoutScope`]/[`PaintScope`] handed to descendants. Dropping the handle
//! unregisters the boundary. Depth comes from the boundary's nesting (`enclosing + 1`), recorded at
//! registration, so a flush re-enters boundaries rootmost-first.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use slotmap::{Key, SlotMap};

use agui_core::deferrable_dirty_list::DeferrableDirtyList;

use crate::{
    context::{LayoutCtx, PaintCtx},
    geometry::Offset,
    paint::{
        compositing::{LayerHandle, OffsetLayer},
        scene::SceneCapacity,
    },
    pipeline::{BoundaryContent, FramePhase, LayoutBuildHost},
    render_object::{box_layout::RenderBox, node::MountedChild},
    semantics::{SemanticsTree, SemanticsTreeBuilder},
};

pub use agui_core::scope::{
    LayoutBoundaryId, LayoutScope, PaintBoundaryId, PaintScope, SemanticsBoundaryId, SemanticsScope,
};

/// A relayout boundary the pipeline re-lays on its own. The pipeline drives it from
/// [`flush_layout`](RenderPipeline::flush_layout) knowing nothing of the layout protocol behind it: the
/// implementor owns the render object and the constraints to re-lay it from, so a box boundary re-lays under
/// the box constraints it recorded and another protocol does the analogous thing for its own.
pub trait LayoutBoundary {
    /// Re-lays this boundary's subtree from the constraints it last recorded. `ctx` is scoped to this
    /// boundary, so a later change within the subtree marks it again.
    fn relayout(&mut self, ctx: &mut LayoutCtx);
}

/// The relayout and repaint boundaries of one widget tree. Shared so a render object can reach it to
/// register and mark a boundary, and the driver can drive a frame's layout and paint.
#[derive(Clone)]
pub struct RenderPipeline {
    inner: Rc<RefCell<Inner>>,
}

struct Inner {
    /// The phase of the frame now running, so a mark whose pipeline already ran this frame is rejected.
    phase: FramePhase,

    layout: SlotMap<LayoutBoundaryId, LayoutBoundaryCell>,
    layout_dirty: DeferrableDirtyList<LayoutBoundaryId>,

    paint: SlotMap<PaintBoundaryId, PaintBoundary>,
    paint_bits: DeferrableDirtyList<PaintBoundaryId>,
    paint_repaint: DeferrableDirtyList<PaintBoundaryId>,
    /// Repaint marks made out of band, each tagged with the phase it owes, applied by `drain_paint_deferred`.
    paint_deferred: Rc<RefCell<Vec<(PaintBoundaryId, PaintPhase)>>>,

    /// A layer's placement changed and the subtree must recomposite, with no boundary to repaint.
    needs_composite: bool,

    /// Fired when the pipeline goes from fully clean to having any pending layout or paint work, so the
    /// driver schedules a frame. One callback: the driver runs a whole frame, the channels decide what reruns.
    notify_needs_frame: Box<dyn Fn()>,

    semantics: SlotMap<SemanticsBoundaryId, SemanticsBoundaryCell>,
    semantics_dirty: DeferrableDirtyList<SemanticsBoundaryId>,
    /// The monotonic source of semantics node ids for this pipeline. A node minted on any walk takes the next
    /// value, so ids stay unique across boundaries and across walks.
    semantics_counter: u64,

    /// Fired when the semantics go from clean to having a marked boundary, so the driver re-reads them. One
    /// pipeline callback. The dirty set names which boundaries changed.
    notify_semantics_update: Box<dyn Fn()>,
}

struct LayoutBoundaryCell {
    /// Depth in the boundary nesting, so a drain re-enters rootmost-first.
    depth: usize,
    /// The boundary, owned here. Taken out for the duration of its own re-lay, so the re-lay can re-enter the
    /// pipeline to register nested boundaries, and put back after.
    boundary: Option<Box<dyn LayoutBoundary>>,
    /// The repaint boundary enclosing this one, marked when this boundary re-lays so the re-laid subtree
    /// repaints.
    paint: PaintScope,
}

struct PaintBoundary {
    depth: usize,
    content: PaintContent,
    layer: LayerHandle<OffsetLayer>,
    /// The buffer lengths the last repaint recorded, sizing the next repaint's buffers.
    paint_capacity: SceneCapacity,
}

struct SemanticsBoundaryCell {
    content: BoundaryContent,
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

impl Default for RenderPipeline {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Inner {
                layout: SlotMap::with_key(),
                layout_dirty: DeferrableDirtyList::default(),

                paint: SlotMap::with_key(),
                paint_bits: DeferrableDirtyList::default(),
                paint_repaint: DeferrableDirtyList::default(),
                needs_composite: false,
                paint_deferred: Rc::new(RefCell::new(Vec::new())),

                phase: FramePhase::Idle,

                notify_needs_frame: Box::new(|| {}),

                semantics: SlotMap::with_key(),
                semantics_dirty: DeferrableDirtyList::default(),
                semantics_counter: 0,

                notify_semantics_update: Box::new(|| {}),
            })),
        }
    }
}

impl RenderPipeline {
    /// Registers `f` to fire when the pipeline goes from fully clean to having pending work, so the driver
    /// schedules a frame.
    pub fn on_needs_frame(&self, f: Box<dyn Fn()>) {
        self.inner.borrow_mut().notify_needs_frame = f;
    }

    /// Fires the needs-frame callback directly, for work that does not go through the pipeline's own
    /// channels: a build-only dirty the owner wants to wake the driver for.
    pub fn request_frame(&self) {
        (self.inner.borrow().notify_needs_frame)();
    }

    /// Sets `phase` as the frame phase now running until the returned guard drops, which restores the
    /// previous phase.
    pub(crate) fn enter_phase(&self, phase: FramePhase) -> PhaseGuard {
        let previous = std::mem::replace(&mut self.inner.borrow_mut().phase, phase);
        PhaseGuard {
            pipeline: self.clone(),
            previous,
        }
    }

    /// Registers `f` to fire when a view's semantics change, so the driver re-reads them.
    pub fn on_needs_semantics_update(&self, f: Box<dyn Fn()>) {
        self.inner.borrow_mut().notify_semantics_update = f;
    }

    /// A deferred marker for `scope`'s boundary, for a render object to mark it from outside a pass.
    #[must_use]
    pub fn deferred_semantics_scope(&self, scope: SemanticsScope) -> DeferredSemanticsScope {
        DeferredSemanticsScope {
            id: scope.0,
            queue: Some(self.inner.borrow().semantics_dirty.deferred_queue()),
        }
    }

    /// Registers a semantics boundary, returning the handle that owns and unregisters it. A
    /// [`View`](crate::view::View) registers its root boundary this way; render objects under it mark it
    /// through a scope the handle hands out.
    pub(crate) fn register_semantics_boundary(
        &self,
        content: BoundaryContent,
    ) -> SemanticsBoundaryHandle {
        let id = self
            .inner
            .borrow_mut()
            .semantics
            .insert(SemanticsBoundaryCell { content });

        SemanticsBoundaryHandle {
            id,
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Marks `scope`'s boundary's semantics changed, firing the pipeline's semantics callback. A reconcile
    /// calls this through [`UpdateCtx`](crate::context::UpdateCtx) when it changes a render object's semantics.
    pub fn mark_needs_semantics_update(&self, scope: SemanticsScope) {
        self.inner.borrow_mut().mark_needs_semantics_update(scope.0);
    }

    /// A deferred handle to `scope`'s boundary, for marking it from a callback that holds no pipeline, such
    /// as a reconcile or a per-frame animation.
    pub fn deferred_layout_scope(&self, scope: LayoutScope) -> DeferredLayoutScope {
        DeferredLayoutScope {
            id: scope.0,
            queue: Some(self.inner.borrow().layout_dirty.deferred_queue()),
        }
    }

    /// Registers `boundary` as a relayout boundary nested under `enclosing`, and returns the handle that owns
    /// and unregisters it. A render object registers its boundary this way the first time it is laid out as
    /// one. Its enclosing repaint boundary is recorded later, at paint, through
    /// [`set_paint_scope`](LayoutBoundaryHandle::set_paint_scope).
    pub(crate) fn register_layout_boundary(
        &self,
        enclosing: LayoutScope,
        boundary: Box<dyn LayoutBoundary>,
    ) -> LayoutBoundaryHandle {
        let mut inner = self.inner.borrow_mut();

        let depth = inner.layout.get(enclosing.0).map_or(0, |b| b.depth + 1);
        let id = inner.layout.insert(LayoutBoundaryCell {
            depth,
            boundary: Some(boundary),
            paint: PaintScope::detached(),
        });

        LayoutBoundaryHandle {
            id,
            inner: Rc::downgrade(&self.inner),
            paint: PaintScope::detached(),
        }
    }

    /// Marks `scope`'s boundary for re-layout before the next frame.
    pub fn mark_needs_layout(&self, scope: LayoutScope) {
        self.inner.borrow_mut().mark_needs_layout(scope.0);
    }

    /// A deferred handle to `scope`'s repaint boundary.
    pub fn deferred_paint_scope(&self, scope: PaintScope) -> DeferredPaintScope {
        DeferredPaintScope {
            id: scope.0,
            queue: Some(Rc::clone(&self.inner.borrow().paint_deferred)),
        }
    }

    /// Registers `content` as a repaint boundary nested under `enclosing`, painting into `layer`, and
    /// returns the handle that owns and unregisters it. The new boundary is left unmarked: the caller either
    /// marks it (the root view) or paints it in the same pass it registers in (an inline boundary).
    pub(crate) fn register_paint_boundary(
        &self,
        enclosing: PaintScope,
        content: PaintContent,
        layer: LayerHandle<OffsetLayer>,
    ) -> PaintBoundaryHandle {
        let mut inner = self.inner.borrow_mut();

        let depth = inner.paint.get(enclosing.0).map_or(0, |b| b.depth + 1);
        let id = inner.paint.insert(PaintBoundary {
            depth,
            content,
            layer,
            paint_capacity: SceneCapacity::default(),
        });

        PaintBoundaryHandle {
            id,
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        let mut inner = self.inner.borrow_mut();
        let was_clean = inner.is_clean();
        inner.mark_needs_paint(scope.0, PaintPhase::CompositingBits);
        inner.mark_needs_paint(scope.0, PaintPhase::Paint);
        if was_clean {
            (inner.notify_needs_frame)();
        }
    }

    /// Marks `scope`'s boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        let mut inner = self.inner.borrow_mut();
        let was_clean = inner.is_clean();
        inner.mark_needs_paint(scope.0, PaintPhase::Paint);
        if was_clean {
            (inner.notify_needs_frame)();
        }
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting any boundary.
    pub fn mark_needs_composite(&self) {
        let mut inner = self.inner.borrow_mut();
        let was_clean = inner.is_clean();
        inner.needs_composite = true;
        if was_clean {
            (inner.notify_needs_frame)();
        }
    }

    /// Re-lays every marked relayout boundary from the constraints it last took, rootmost-first, leaving the
    /// rest untouched.
    pub(crate) fn flush_layout(&self, host: &RefCell<LayoutBuildHost>) {
        let _phase = self.enter_phase(FramePhase::Layout);

        self.inner.borrow_mut().take_layout_dirty();

        let mut i = 0;
        loop {
            let id = match self.inner.borrow().layout_dirty.drained().get(i).copied() {
                Some(id) => id,
                None => break,
            };
            i += 1;

            let (boundary, paint) = {
                let mut inner = self.inner.borrow_mut();

                // A boundary dropped since it was drained is absent, so skip it.
                let Some(cell) = inner.layout.get_mut(id) else {
                    continue;
                };

                (cell.boundary.take(), cell.paint)
            };

            let Some(mut boundary) = boundary else {
                continue;
            };

            let mut ctx = LayoutCtx::with_host(self, LayoutScope(id), host);
            boundary.relayout(&mut ctx);

            // Put the boundary back, unless its own re-lay unregistered it.
            if let Some(cell) = self.inner.borrow_mut().layout.get_mut(id) {
                cell.boundary = Some(boundary);
            }

            // The boundary's painting is now stale, so repaint the boundary that encloses it.
            self.mark_needs_paint(paint);
        }
    }

    /// Re-walks each semantics boundary marked since the last frame and hands its freshly built
    /// [`SemanticsTree`] to `update`, then re-arms so the next change fires the callback again.
    pub fn flush_semantics(&self, mut update: impl FnMut(SemanticsBoundaryId, SemanticsTree)) {
        self.inner.borrow_mut().take_semantics_dirty();

        let mut counter = self.inner.borrow().semantics_counter;

        let mut i = 0;
        loop {
            let id = match self.inner.borrow().semantics_dirty.drained().get(i).copied() {
                Some(id) => id,
                None => break,
            };
            i += 1;

            let content = match self.inner.borrow().semantics.get(id) {
                Some(boundary) => Rc::clone(&boundary.content),
                None => continue,
            };

            let mut builder = SemanticsTreeBuilder::new(&mut counter);
            content.borrow_mut().dyn_build_semantics(&mut builder);
            let tree = SemanticsTree::new(builder.finish());

            update(id, tree);
        }

        self.inner.borrow_mut().semantics_counter = counter;
    }

    /// Recomputes compositing bits, repaints, and recomposites the marked repaint boundaries.
    pub fn flush_paint(&self) {
        let paint: Vec<(PaintBoundaryId, PaintPhase)> = self
            .inner
            .borrow()
            .paint_deferred
            .borrow_mut()
            .drain(..)
            .collect();

        for (id, phase) in paint {
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

        let _phase = self.enter_phase(FramePhase::Composite);
        self.inner.borrow_mut().needs_composite = false;
    }

    fn flush_compositing_bits(&self) {
        let _phase = self.enter_phase(FramePhase::CompositingBits);

        self.inner
            .borrow_mut()
            .take_paint_dirty(PaintPhase::CompositingBits);

        let mut i = 0;
        loop {
            let id = match self.inner.borrow().paint_bits.drained().get(i).copied() {
                Some(id) => id,
                None => break,
            };
            i += 1;

            let content = match self.inner.borrow().paint.get(id) {
                Some(boundary) => boundary.content.clone(),
                None => continue,
            };

            content.update_compositing_bits();
        }
    }

    fn flush_repaint(&self) {
        let _phase = self.enter_phase(FramePhase::Paint);

        self.inner.borrow_mut().take_paint_dirty(PaintPhase::Paint);

        let mut i = 0;
        loop {
            let id = match self.inner.borrow().paint_repaint.drained().get(i).copied() {
                Some(id) => id,
                None => break,
            };
            i += 1;

            let (content, layer, capacity) = match self.inner.borrow().paint.get(id) {
                Some(boundary) => (
                    boundary.content.clone(),
                    boundary.layer.clone(),
                    boundary.paint_capacity,
                ),
                None => continue,
            };

            layer.borrow_mut().clear();

            let recorded =
                PaintCtx::paint_with_capacity(&layer, capacity, self, PaintScope(id), |ctx| {
                    content.paint(ctx, Offset::ZERO);
                });

            if let Some(boundary) = self.inner.borrow_mut().paint.get_mut(id) {
                boundary.paint_capacity = recorded;
            }
        }
    }
}

/// Restores the [`FramePhase`] that was running when [`RenderPipeline::enter_phase`] was called.
pub(crate) struct PhaseGuard {
    pipeline: RenderPipeline,
    previous: FramePhase,
}

impl Drop for PhaseGuard {
    fn drop(&mut self) {
        self.pipeline.inner.borrow_mut().phase = self.previous;
    }
}

impl Inner {
    /// Whether the pipeline has no pending layout or paint work.
    fn is_clean(&self) -> bool {
        self.layout_dirty.is_clean()
            && self.paint_bits.is_clean()
            && self.paint_repaint.is_clean()
            && !self.needs_composite
    }

    fn mark_needs_layout(&mut self, id: LayoutBoundaryId) {
        self.phase.assert_can_mark(FramePhase::Layout);

        if !self.layout.contains_key(id) {
            return;
        }

        let was_clean = self.is_clean();
        self.layout_dirty.mark(id);

        if was_clean {
            (self.notify_needs_frame)();
        }
    }

    /// Drains the marked relayout boundaries into the list's buffer, deduplicated and sorted shallowest-first,
    /// to read through `layout_dirty.drained()`.
    fn take_layout_dirty(&mut self) {
        let cells = &self.layout;
        let out = self.layout_dirty.take_dirty();
        out.sort_unstable();
        out.dedup();
        out.sort_by_key(|&id| cells.get(id).map_or(0, |b| b.depth));
    }

    fn mark_needs_semantics_update(&mut self, id: SemanticsBoundaryId) {
        if !self.semantics.contains_key(id) {
            return;
        }

        let was_clean = self.semantics_dirty.is_clean();
        self.semantics_dirty.mark(id);

        if was_clean {
            (self.notify_semantics_update)();
        }
    }

    /// Drains the marked semantics boundaries into the list's buffer, deduplicated, to read through
    /// `semantics_dirty.drained()`.
    fn take_semantics_dirty(&mut self) {
        let out = self.semantics_dirty.take_dirty();
        out.sort_unstable();
        out.dedup();
    }

    fn mark_needs_paint(&mut self, id: PaintBoundaryId, phase: PaintPhase) {
        self.phase.assert_can_mark(match phase {
            PaintPhase::CompositingBits => FramePhase::CompositingBits,
            PaintPhase::Paint => FramePhase::Paint,
            PaintPhase::Composite => FramePhase::Composite,
        });

        if !self.paint.contains_key(id) {
            return;
        }

        match phase {
            PaintPhase::CompositingBits => {
                self.paint_bits.mark(id);
            }
            PaintPhase::Paint => {
                self.paint_repaint.mark(id);
            }
            PaintPhase::Composite => {}
        }
    }

    /// Drains the repaint boundaries marked for `phase` into that list's buffer, deduplicated, to read through
    /// its `drained()`.
    fn take_paint_dirty(&mut self, phase: PaintPhase) {
        let list = match phase {
            PaintPhase::CompositingBits => &mut self.paint_bits,
            PaintPhase::Paint => &mut self.paint_repaint,
            PaintPhase::Composite => return,
        };

        let out = list.take_dirty();
        out.sort_unstable();
        out.dedup();
    }
}

/// The sole owner of a registered relayout boundary, held by the render object that established it. Dropping
/// it unregisters the boundary. Hand descendants the [`scope`](Self::scope) to mark, and request a re-layout
/// of the boundary itself with [`mark_needs_layout`](Self::mark_needs_layout).
pub struct LayoutBoundaryHandle {
    id: LayoutBoundaryId,
    inner: Weak<RefCell<Inner>>,
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
    pub fn replace(&mut self, boundary: Box<dyn LayoutBoundary>) {
        if let Some(inner) = self.inner.upgrade()
            && let Some(cell) = inner.borrow_mut().layout.get_mut(self.id)
        {
            cell.boundary = Some(boundary);
        }
    }

    /// Records the repaint boundary enclosing this one, so a re-lay can mark it for repaint. The render object
    /// that established this boundary calls it during paint, where the enclosing repaint boundary is known.
    pub fn set_paint_scope(&mut self, paint: PaintScope) {
        if self.paint == paint {
            return;
        }

        self.paint = paint;

        if let Some(inner) = self.inner.upgrade()
            && let Some(cell) = inner.borrow_mut().layout.get_mut(self.id)
        {
            cell.paint = paint;
        }
    }

    /// Requests that this boundary be re-laid before the next frame.
    pub fn mark_needs_layout(&self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.borrow_mut().mark_needs_layout(self.id);
        }
    }
}

impl Drop for LayoutBoundaryHandle {
    fn drop(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };

        // A pending mark for this id is left in the dirty and deferred lists: the next flush drops it when it
        // finds no cell.
        let removed = inner.borrow_mut().layout.remove(self.id);

        // Drop the removed cell only after the borrow is released: it may own a nested boundary (a child
        // render object's), whose own drop re-borrows the pipeline to unregister.
        drop(removed);
    }
}

/// The sole owner of a registered repaint boundary, held by the render object that established it. Dropping
/// it unregisters the boundary and discards its layer. Hand descendants the mark-only [`scope`](Self::scope),
/// and mark the boundary directly through this handle.
pub struct PaintBoundaryHandle {
    id: PaintBoundaryId,
    inner: Weak<RefCell<Inner>>,
}

impl PaintBoundaryHandle {
    pub fn scope(&self) -> PaintScope {
        PaintScope(self.id)
    }

    /// Marks this boundary's compositing bits for recomputation before its next repaint, and the boundary
    /// for repaint.
    pub fn mark_needs_compositing_bits_update(&self) {
        if let Some(inner) = self.inner.upgrade() {
            let mut inner = inner.borrow_mut();
            let was_clean = inner.is_clean();
            inner.mark_needs_paint(self.id, PaintPhase::CompositingBits);
            inner.mark_needs_paint(self.id, PaintPhase::Paint);
            if was_clean {
                (inner.notify_needs_frame)();
            }
        }
    }

    /// Marks this boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self) {
        if let Some(inner) = self.inner.upgrade() {
            let mut inner = inner.borrow_mut();
            let was_clean = inner.is_clean();
            inner.mark_needs_paint(self.id, PaintPhase::Paint);
            if was_clean {
                (inner.notify_needs_frame)();
            }
        }
    }

    /// Schedules a recomposite of the subtree on the next frame, without repainting this boundary.
    pub fn mark_needs_composite(&self) {
        if let Some(inner) = self.inner.upgrade() {
            let mut inner = inner.borrow_mut();
            let was_clean = inner.is_clean();
            inner.needs_composite = true;
            if was_clean {
                (inner.notify_needs_frame)();
            }
        }
    }
}

impl Drop for PaintBoundaryHandle {
    fn drop(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };

        // As for a layout boundary: the removed content's own drop can re-enter this borrow, so release it
        // before the content drops.
        let removed = {
            let mut inner = inner.borrow_mut();
            inner
                .paint_deferred
                .borrow_mut()
                .retain(|(queued, _)| *queued != self.id);
            inner.paint.remove(self.id)
        };
        drop(removed);
    }
}

/// The sole owner of a registered semantics boundary, held by whatever established it. Dropping it
/// unregisters the boundary. Hand descendants the [`scope`](Self::scope) to mark it, and mark the boundary
/// itself through [`mark_needs_semantics_update`](Self::mark_needs_semantics_update).
pub struct SemanticsBoundaryHandle {
    id: SemanticsBoundaryId,
    inner: Weak<RefCell<Inner>>,
}

impl SemanticsBoundaryHandle {
    /// Marks this boundary's semantics changed, firing the pipeline's semantics callback so the driver
    /// re-reads this view.
    pub fn mark_needs_semantics_update(&self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.borrow_mut().mark_needs_semantics_update(self.id);
        }
    }

    /// This boundary's scope, for the view to thread down its subtree so descendants can mark it.
    #[must_use]
    pub fn scope(&self) -> SemanticsScope {
        SemanticsScope(self.id)
    }

    /// A deferred marker for this boundary, captured by a descendant render object to request a re-read
    /// when its semantics later change.
    #[must_use]
    pub fn deferred_scope(&self) -> DeferredSemanticsScope {
        let queue = self
            .inner
            .upgrade()
            .map(|inner| inner.borrow().semantics_dirty.deferred_queue());

        DeferredSemanticsScope { id: self.id, queue }
    }

    /// Runs `f` with the pipeline's semantics id counter, so a full walk through the view mints from the same
    /// source as the per-boundary flush. The counter is copied out and written back, so the pipeline is not
    /// borrowed while `f` runs.
    pub(crate) fn with_counter<R>(&self, f: impl FnOnce(&mut u64) -> R) -> R {
        let inner = self
            .inner
            .upgrade()
            .expect("the pipeline outlives the view");

        let mut counter = inner.borrow().semantics_counter;
        let result = f(&mut counter);
        inner.borrow_mut().semantics_counter = counter;

        result
    }
}

impl Drop for SemanticsBoundaryHandle {
    fn drop(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };

        // A pending mark for this id is left in the dirty and deferred lists: the next flush drops it when it
        // finds no cell.
        let removed = inner.borrow_mut().semantics.remove(self.id);

        // Drop the removed cell only after the borrow is released: it holds the boundary's render object,
        // whose drop may re-borrow the pipeline to unregister nested boundaries.
        drop(removed);
    }
}

/// A captured route to mark a semantics boundary from outside a pipeline pass, such as a property setter or
/// an animation tick. The request is applied on the pipeline's next frame. A detached scope, or one whose
/// boundary is gone, marks nothing.
#[derive(Clone)]
pub struct DeferredSemanticsScope {
    id: SemanticsBoundaryId,
    queue: Option<Rc<RefCell<Vec<SemanticsBoundaryId>>>>,
}

impl Default for DeferredSemanticsScope {
    fn default() -> Self {
        Self::detached()
    }
}

impl DeferredSemanticsScope {
    /// A scope that marks no boundary.
    #[must_use]
    pub fn detached() -> Self {
        Self {
            id: SemanticsBoundaryId::null(),
            queue: None,
        }
    }

    /// Queues this boundary's semantics to be re-read on the pipeline's next frame.
    pub fn mark_needs_semantics_update(&self) {
        if let Some(queue) = &self.queue {
            queue.borrow_mut().push(self.id);
        }
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
