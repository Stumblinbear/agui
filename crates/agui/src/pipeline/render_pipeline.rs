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

use slotmap::{Key, SlotMap, new_key_type};

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

new_key_type! {
    /// Identifies a relayout boundary within one pipeline.
    pub struct LayoutBoundaryId;
    /// Identifies a repaint boundary within one pipeline.
    pub struct PaintBoundaryId;
    /// Identifies a semantics boundary within one pipeline.
    pub struct SemanticsBoundaryId;
}

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
    /// The relayout boundaries waiting to be re-laid, with a per-boundary `enrolled` flag preventing a
    /// second mark from enrolling one twice.
    layout_dirty: Vec<LayoutBoundaryId>,
    layout_deferred: Rc<RefCell<Vec<LayoutBoundaryId>>>,
    layout_scratch: Vec<LayoutBoundaryId>,

    paint: SlotMap<PaintBoundaryId, PaintBoundary>,
    paint_bits_dirty: Vec<PaintBoundaryId>,
    paint_dirty: Vec<PaintBoundaryId>,
    paint_deferred: Rc<RefCell<Vec<(PaintBoundaryId, PaintPhase)>>>,
    paint_scratch: Vec<PaintBoundaryId>,

    /// A layer's placement changed and the subtree must recomposite, with no boundary to repaint.
    needs_composite: bool,

    /// Fired when the pipeline goes from fully clean to having any pending layout or paint work, so the
    /// driver schedules a frame. One callback: the driver runs a whole frame, the channels decide what reruns.
    notify_needs_frame: Box<dyn Fn()>,

    semantics: SlotMap<SemanticsBoundaryId, SemanticsBoundaryCell>,
    semantics_dirty: Vec<SemanticsBoundaryId>,
    semantics_deferred: Rc<RefCell<Vec<SemanticsBoundaryId>>>,
    semantics_scratch: Vec<SemanticsBoundaryId>,
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
    /// Awaiting re-layout; cleared when the boundary is re-laid.
    needs_layout: bool,
    /// In `layout_dirty`, so a second mark does not enroll it twice.
    enrolled: bool,
}

struct PaintBoundary {
    depth: usize,
    content: PaintContent,
    layer: LayerHandle<OffsetLayer>,
    /// The buffer lengths the last repaint recorded, sizing the next repaint's buffers.
    paint_capacity: SceneCapacity,
    /// Bit 0: in `paint_bits_dirty`. Bit 1: in `paint_dirty`. Keeps a second mark from enrolling twice.
    enrolled: u8,
}

struct SemanticsBoundaryCell {
    content: BoundaryContent,

    /// In `semantics_dirty`, so a second mark does not enroll it twice.
    enrolled: bool,
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
                layout_dirty: Vec::new(),
                layout_deferred: Rc::new(RefCell::new(Vec::new())),
                layout_scratch: Vec::new(),

                paint: SlotMap::with_key(),
                paint_bits_dirty: Vec::new(),
                paint_dirty: Vec::new(),
                needs_composite: false,
                paint_deferred: Rc::new(RefCell::new(Vec::new())),
                paint_scratch: Vec::new(),

                phase: FramePhase::Idle,

                notify_needs_frame: Box::new(|| {}),

                semantics: SlotMap::with_key(),
                semantics_dirty: Vec::new(),
                semantics_deferred: Rc::new(RefCell::new(Vec::new())),
                semantics_scratch: Vec::new(),
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
            queue: Some(Rc::clone(&self.inner.borrow().semantics_deferred)),
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
            .insert(SemanticsBoundaryCell {
                content,
                enrolled: false,
            });

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
            queue: Some(Rc::clone(&self.inner.borrow().layout_deferred)),
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
            needs_layout: false,
            enrolled: false,
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
            enrolled: 0,
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

    /// Applies every out-of-band mark queued since the last drain. Called once at the start of a frame,
    /// before the boundaries are re-laid and repainted.
    pub fn drain_deferred(&self) {
        let (layout_queue, paint_queue) = {
            let inner = self.inner.borrow();
            (
                Rc::clone(&inner.layout_deferred),
                Rc::clone(&inner.paint_deferred),
            )
        };

        let layout: Vec<LayoutBoundaryId> = layout_queue.borrow_mut().drain(..).collect();
        for id in layout {
            self.inner.borrow_mut().mark_needs_layout(id);
        }

        let paint: Vec<(PaintBoundaryId, PaintPhase)> =
            paint_queue.borrow_mut().drain(..).collect();
        for (id, phase) in paint {
            match phase {
                PaintPhase::Paint => self.mark_needs_paint(PaintScope(id)),
                PaintPhase::CompositingBits => {
                    self.mark_needs_compositing_bits_update(PaintScope(id));
                }
                PaintPhase::Composite => self.mark_needs_composite(),
            }
        }

        let semantics_queue = Rc::clone(&self.inner.borrow().semantics_deferred);
        let semantics: Vec<SemanticsBoundaryId> = semantics_queue.borrow_mut().drain(..).collect();
        for id in semantics {
            self.inner.borrow_mut().mark_needs_semantics_update(id);
        }
    }

    /// Re-lays every marked relayout boundary from the constraints it last took, rootmost-first, leaving the
    /// rest untouched.
    pub(crate) fn flush_layout(&self, host: &RefCell<LayoutBuildHost>) {
        let _phase = self.enter_phase(FramePhase::Layout);

        let mut scratch = {
            let mut inner = self.inner.borrow_mut();
            inner.take_layout_dirty()
        };

        for &id in &scratch {
            let (boundary, paint) = {
                let mut inner = self.inner.borrow_mut();

                // A boundary dropped since it was drained is absent, so skip it; an enclosing boundary's
                // relayout may also have covered this one, clearing its pending mark in place.
                let Some(cell) = inner.layout.get_mut(id) else {
                    continue;
                };
                if !std::mem::replace(&mut cell.needs_layout, false) {
                    continue;
                }

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

        let mut inner = self.inner.borrow_mut();
        scratch.clear();
        inner.layout_scratch = scratch;
    }

    /// Re-walks each semantics boundary marked since the last frame and hands its freshly built
    /// [`SemanticsTree`] to `update`, then re-arms so the next change fires the callback again.
    pub fn flush_semantics(&self, mut update: impl FnMut(SemanticsBoundaryId, SemanticsTree)) {
        let mut scratch = self.inner.borrow_mut().take_semantics_dirty();

        let mut counter = self.inner.borrow().semantics_counter;

        for &id in &scratch {
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

        scratch.clear();
        self.inner.borrow_mut().semantics_scratch = scratch;
    }

    /// Recomputes compositing bits, repaints, and recomposites the marked repaint boundaries.
    pub fn flush_paint(&self) {
        let mut scratch = {
            let mut inner = self.inner.borrow_mut();
            std::mem::take(&mut inner.paint_scratch)
        };

        self.flush_compositing_bits(&mut scratch);
        self.flush_repaint(&mut scratch);

        {
            let _phase = self.enter_phase(FramePhase::Composite);
            self.inner.borrow_mut().needs_composite = false;
        }

        self.inner.borrow_mut().paint_scratch = scratch;
    }

    fn flush_compositing_bits(&self, scratch: &mut Vec<PaintBoundaryId>) {
        let _phase = self.enter_phase(FramePhase::CompositingBits);

        self.inner
            .borrow_mut()
            .take_paint_dirty(PaintPhase::CompositingBits, scratch);

        for &id in &*scratch {
            let content = match self.inner.borrow().paint.get(id) {
                Some(boundary) => boundary.content.clone(),
                None => continue,
            };

            content.update_compositing_bits();
        }
    }

    fn flush_repaint(&self, scratch: &mut Vec<PaintBoundaryId>) {
        let _phase = self.enter_phase(FramePhase::Paint);

        self.inner
            .borrow_mut()
            .take_paint_dirty(PaintPhase::Paint, scratch);

        for &id in &*scratch {
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
    /// Whether the pipeline has no pending work on any channel.
    fn is_clean(&self) -> bool {
        self.layout_dirty.is_empty()
            && self.paint_bits_dirty.is_empty()
            && self.paint_dirty.is_empty()
            && !self.needs_composite
    }

    fn mark_needs_layout(&mut self, id: LayoutBoundaryId) {
        self.phase.assert_can_mark(FramePhase::Layout);

        let was_clean = self.is_clean();

        let Some(boundary) = self.layout.get_mut(id) else {
            return;
        };

        boundary.needs_layout = true;

        if boundary.enrolled {
            return;
        }

        boundary.enrolled = true;

        self.layout_dirty.push(id);

        if was_clean {
            (self.notify_needs_frame)();
        }
    }

    /// Drains the relayout boundaries waiting, clearing their enrollment, sorted shallowest-first.
    fn take_layout_dirty(&mut self) -> Vec<LayoutBoundaryId> {
        let mut out = std::mem::take(&mut self.layout_scratch);
        out.clear();
        std::mem::swap(&mut self.layout_dirty, &mut out);

        for &id in &out {
            if let Some(boundary) = self.layout.get_mut(id) {
                boundary.enrolled = false;
            }
        }

        let cells = &self.layout;
        out.sort_by_key(|&id| cells.get(id).map_or(0, |b| b.depth));
        out
    }

    fn mark_needs_semantics_update(&mut self, id: SemanticsBoundaryId) {
        let was_clean = self.semantics_dirty.is_empty();

        let Some(boundary) = self.semantics.get_mut(id) else {
            return;
        };

        if boundary.enrolled {
            return;
        }

        boundary.enrolled = true;

        self.semantics_dirty.push(id);

        if was_clean {
            (self.notify_semantics_update)();
        }
    }

    /// Drains the semantics boundaries marked, clearing their enrollment.
    fn take_semantics_dirty(&mut self) -> Vec<SemanticsBoundaryId> {
        let mut out = std::mem::take(&mut self.semantics_scratch);
        out.clear();
        std::mem::swap(&mut self.semantics_dirty, &mut out);

        for &id in &out {
            if let Some(boundary) = self.semantics.get_mut(id) {
                boundary.enrolled = false;
            }
        }

        out
    }

    fn mark_needs_paint(&mut self, id: PaintBoundaryId, phase: PaintPhase) {
        self.phase.assert_can_mark(match phase {
            PaintPhase::CompositingBits => FramePhase::CompositingBits,
            PaintPhase::Paint => FramePhase::Paint,
            PaintPhase::Composite => FramePhase::Composite,
        });

        let bit = match phase {
            PaintPhase::CompositingBits => 0b01,
            PaintPhase::Paint => 0b10,
            PaintPhase::Composite => return,
        };

        let Some(boundary) = self.paint.get_mut(id) else {
            return;
        };

        if boundary.enrolled & bit != 0 {
            return;
        }

        boundary.enrolled |= bit;

        match phase {
            PaintPhase::CompositingBits => self.paint_bits_dirty.push(id),
            PaintPhase::Paint => self.paint_dirty.push(id),
            PaintPhase::Composite => {}
        }
    }

    fn take_paint_dirty(&mut self, phase: PaintPhase, out: &mut Vec<PaintBoundaryId>) {
        let (channel, bit) = match phase {
            PaintPhase::CompositingBits => (&mut self.paint_bits_dirty, 0b01),
            PaintPhase::Paint => (&mut self.paint_dirty, 0b10),
            PaintPhase::Composite => return,
        };

        out.clear();
        std::mem::swap(channel, out);

        for &id in &*out {
            if let Some(boundary) = self.paint.get_mut(id) {
                boundary.enrolled &= !bit;
            }
        }

        let cells = &self.paint;
        out.sort_by_key(|&id| cells.get(id).map_or(0, |b| b.depth));
    }
}

/// Names the relayout boundary a render object is laid out under: an id into the pipeline, threaded down
/// through layout. A node forwards it to the children it lays out and stores it to request a relayout later.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct LayoutScope(pub(crate) LayoutBoundaryId);

impl LayoutScope {
    /// A scope detached from any pipeline, which names no boundary, so registering under it registers at the
    /// root and a node laid out under it is not itself a boundary.
    pub fn detached() -> Self {
        Self(LayoutBoundaryId::null())
    }

    /// Whether this scope names no boundary.
    pub fn is_detached(&self) -> bool {
        self.0.is_null()
    }
}

/// Names the repaint boundary a render object paints into: an id into the pipeline, threaded down through
/// paint. A node holds the scope of its nearest enclosing boundary to repaint it when its painting goes
/// stale.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PaintScope(pub(crate) PaintBoundaryId);

impl PaintScope {
    /// A scope that names no boundary.
    pub fn detached() -> Self {
        Self(PaintBoundaryId::null())
    }

    /// Whether this scope names no boundary.
    pub fn is_detached(&self) -> bool {
        self.0.is_null()
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

        let removed = {
            let mut inner = inner.borrow_mut();
            inner
                .layout_deferred
                .borrow_mut()
                .retain(|queued| *queued != self.id);
            inner.layout.remove(self.id)
        };

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
            .map(|inner| Rc::clone(&inner.borrow().semantics_deferred));

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

        let removed = {
            let mut inner = inner.borrow_mut();
            inner
                .semantics_deferred
                .borrow_mut()
                .retain(|id| *id != self.id);
            inner.semantics_dirty.retain(|id| *id != self.id);
            inner.semantics.remove(self.id)
        };

        // Drop the removed cell only after the borrow is released: it holds the boundary's render object,
        // whose drop may re-borrow the pipeline to unregister nested boundaries.
        drop(removed);
    }
}

/// Names the semantics boundary a render object marks: an id into the pipeline, threaded down through build.
/// A node holds the scope of its enclosing boundary to mark it during a reconcile.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SemanticsScope(pub(crate) SemanticsBoundaryId);

impl Default for SemanticsScope {
    fn default() -> Self {
        Self::detached()
    }
}

impl SemanticsScope {
    /// A scope that names no boundary.
    #[must_use]
    pub fn detached() -> Self {
        Self(SemanticsBoundaryId::null())
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
