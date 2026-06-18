use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use slotmap::Key;

use crate::{
    context::LayoutCtx,
    pipeline::{
        BoundaryContent, FramePhase, enter_phase,
        paint::{PaintPipeline, PaintScope},
    },
    reactor::{Node, NodeId, Reactor},
    render_object::box_layout::{BoxConstraints, RenderBox},
};

/// A relayout boundary as the reactor sees it: opaque content laid out from cached constraints.
struct LayoutNode {
    /// The depth in the boundary nesting, so a drain re-enters rootmost-first.
    depth: usize,

    content: BoundaryContent,

    /// The constraints this boundary was last laid out under, replayed to re-lay it on its own. A
    /// boundary constrained from outside the tree has them written by the owner.
    constraints: Option<BoxConstraints>,

    /// The repaint boundary enclosing this one, marked when this boundary re-lays so the re-laid subtree
    /// repaints.
    paint: PaintScope,

    /// Whether this boundary is awaiting re-layout. It outlives the reactor enrollment, and clears when
    /// the boundary is re-laid, whether by the flush or in place by the boundary above it.
    needs_layout: bool,
}

impl Node for LayoutNode {
    type Reaction = ();

    fn depth(&self) -> usize {
        self.depth
    }
}

/// Lays out the relayout boundaries of one subtree, re-laying only the ones that changed.
///
/// The subtree's root is one boundary; every boundary nested inside is re-laid from the constraints it
/// last took. A change confined to a boundary re-lays just that boundary, leaving the rest untouched.
/// Mark a boundary through the [`LayoutScope`] it was given.
pub struct LayoutPipeline {
    inner: Rc<RefCell<LayoutInner>>,
}

impl Default for LayoutPipeline {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(LayoutInner {
                reactor: Reactor::default(),
                deferred: Rc::new(RefCell::new(Vec::new())),
                scratch: Vec::new(),
                notify: Box::new(|| {}),
                in_layout: false,
            })),
        }
    }
}

impl LayoutPipeline {
    pub fn on_needs_layout(&mut self, f: Box<dyn Fn()>) {
        self.inner.borrow_mut().notify = f;
    }

    /// Registers `content` as a relayout boundary nested under `enclosing`, enclosed by `paint`, and
    /// returns the handle that owns and marks it. A boundary established during layout is registered this
    /// way so its descendants mark it rather than the boundary above. A detached or removed enclosing
    /// scope registers at depth zero.
    pub fn register_boundary(
        &self,
        enclosing: LayoutScope,
        content: BoundaryContent,
        paint: PaintScope,
    ) -> RegisteredLayoutBoundary {
        let depth = match self.inner.borrow().reactor.get(enclosing.0) {
            Some(node) => node.depth + 1,
            None => 0,
        };

        let node_id = self.inner.borrow_mut().reactor.register(LayoutNode {
            depth,
            content,
            constraints: None,
            paint,
            needs_layout: false,
        });

        RegisteredLayoutBoundary {
            node_id,
            inner: Rc::downgrade(&self.inner),
        }
    }

    /// Marks `scope`'s boundary for re-layout before the next frame.
    pub fn mark_needs_layout(&self, scope: LayoutScope) {
        self.inner.borrow_mut().mark_needs_layout(scope.0);
    }

    /// A deferred handle to `scope`'s boundary, for marking it from a callback that runs with no pipeline
    /// in hand, such as a reconcile.
    pub fn deferred_scope(&self, scope: LayoutScope) -> DeferredLayoutScope {
        DeferredLayoutScope {
            queue: Some(Rc::clone(&self.inner.borrow().deferred)),
            node_id: scope.0,
        }
    }

    /// Applies every out-of-band mark queued since the last drain. Called once at the start of a frame,
    /// before the boundaries are re-laid.
    pub fn drain_deferred(&mut self) {
        let queue = Rc::clone(&self.inner.borrow().deferred);
        let drained: Vec<NodeId> = queue.borrow_mut().drain(..).collect();

        for node_id in drained {
            self.mark_needs_layout(LayoutScope(node_id));
        }
    }

    /// Re-lays every marked boundary from the constraints it last took, rootmost-first, leaving the rest
    /// untouched.
    pub fn flush(&self, paint_pipeline: &mut PaintPipeline) {
        let _phase = enter_phase(FramePhase::Layout);

        let mut scratch = {
            let mut inner = self.inner.borrow_mut();
            inner.in_layout = true;

            let mut scratch = std::mem::take(&mut inner.scratch);
            inner.reactor.take_dirty((), &mut scratch);
            scratch
        };

        for &node_id in &scratch {
            let (constraints, mut content, paint) = {
                let mut inner = self.inner.borrow_mut();

                // A boundary dropped since it was drained is absent from the reactor, so skip it rather
                // than replay its stale constraints. An enclosing boundary's relayout may also have
                // covered this one, clearing its pending mark in place, so skip that too.
                let Some(node) = inner.reactor.get_mut(node_id) else {
                    continue;
                };
                if !std::mem::replace(&mut node.needs_layout, false) {
                    continue;
                }
                let Some(constraints) = node.constraints else {
                    continue;
                };

                (constraints, Rc::clone(&node.content), node.paint)
            };

            let mut ctx = LayoutCtx::new(self, paint_pipeline, LayoutScope(node_id));
            content.layout(&mut ctx, constraints);

            // Layout reconciles the subtree under a boundary, never the boundary itself, so the boundary
            // it was invoked for cannot have been dropped by its own layout.
            debug_assert!(
                self.inner.borrow().reactor.get(node_id).is_some(),
                "a boundary must not be dropped during its own layout"
            );

            // The boundary's painting is now stale, so repaint the boundary that encloses it.
            paint_pipeline.mark_needs_paint(paint);
        }

        let mut inner = self.inner.borrow_mut();
        scratch.clear();
        inner.scratch = scratch;
        inner.in_layout = false;
    }
}

/// The layout dirty state, shared so a [`LayoutScope`] can mark a boundary out of band.
struct LayoutInner {
    reactor: Reactor<LayoutNode, 1>,

    /// The out-of-band marks queued by [`DeferredLayoutScope`]s since the last drain.
    deferred: Rc<RefCell<Vec<NodeId>>>,

    /// Reused across flushes to hold the drained ids, so its capacity survives between frames.
    scratch: Vec<NodeId>,

    notify: Box<dyn Fn()>,

    in_layout: bool,
}

impl LayoutInner {
    /// Marks the boundary `node_id` names for re-layout, firing the schedule hook on the clean-to-dirty
    /// edge. A node that is gone marks nothing.
    ///
    /// # Panics
    ///
    /// Panics if called during a layout pass, since the dirty set is being drained, or once the paint
    /// phase is flushing, which layout precedes.
    fn mark_needs_layout(&mut self, node_id: NodeId) {
        FramePhase::assert_can_mark(FramePhase::Layout);

        assert!(
            !self.in_layout,
            "cannot request layout while layout is in progress"
        );

        let Some(node) = self.reactor.get_mut(node_id) else {
            return;
        };
        node.needs_layout = true;

        tracing::trace!(boundary = ?node_id, "marked boundary for re-layout");

        let was_clean = self.reactor.is_clean();
        self.reactor.mark(node_id, ());

        if was_clean {
            (self.notify)();
        }
    }
}

/// The sole owner of a registered relayout boundary, held by the render object that established it.
///
/// While this handle is alive the boundary is registered and can be re-laid on its own; dropping it
/// unregisters the boundary and clears any pending re-layout of it. Hand descendants the [`scope`] to
/// mark, record the constraints the boundary takes with [`update_constraints`], and request a re-layout
/// with [`mark_needs_layout`].
///
/// [`scope`]: Self::scope
/// [`update_constraints`]: Self::update_constraints
/// [`mark_needs_layout`]: Self::mark_needs_layout
pub struct RegisteredLayoutBoundary {
    node_id: NodeId,
    inner: Weak<RefCell<LayoutInner>>,
}

impl RegisteredLayoutBoundary {
    pub fn scope(&self) -> LayoutScope {
        LayoutScope(self.node_id)
    }

    /// Records the constraints this boundary is re-laid under, without marking it. A boundary re-laid in
    /// place by its parent keeps its cached constraints current this way, and that relayout satisfies
    /// any re-layout still pending on the boundary.
    pub fn update_constraints(&self, constraints: BoxConstraints) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };

        if let Some(node) = inner.borrow_mut().reactor.get_mut(self.node_id) {
            node.constraints = Some(constraints);
            node.needs_layout = false;
        }
    }

    /// Records the constraints this boundary is re-laid under and marks it. A boundary constrained from
    /// outside the tree is sized this way, on mount and whenever those constraints change.
    pub fn set_constraints(&self, constraints: BoxConstraints) {
        self.update_constraints(constraints);
        self.mark_needs_layout();
    }

    /// Requests that this boundary be re-laid-out before the next frame. On the clean-to-dirty edge of
    /// the layout channel, the schedule hook fires so the driver schedules a frame.
    ///
    /// # Panics
    ///
    /// Panics if called during a layout pass, since the dirty set is being drained.
    pub fn mark_needs_layout(&self) {
        if let Some(inner) = self.inner.upgrade() {
            inner.borrow_mut().mark_needs_layout(self.node_id);
        }
    }
}

impl Drop for RegisteredLayoutBoundary {
    fn drop(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };

        // The removed node owns the boundary's render content, whose own drop can unregister a nested
        // boundary and so re-enter this borrow. Hold the node until the borrow is released, then let it
        // drop. Removing it makes the boundary absent: a flush that already drained its id skips it,
        // rather than replaying its stale constraints.
        let removed = {
            let mut inner = inner.borrow_mut();
            let removed = inner.reactor.remove(self.node_id);
            inner
                .deferred
                .borrow_mut()
                .retain(|queued| *queued != self.node_id);
            removed
        };

        drop(removed);
    }
}

/// Names the relayout boundary a render object is laid out under. Unlike a paint scope, which a node
/// captures once at mount, a layout scope is threaded through layout, because which node bounds a
/// relayout is decided during layout from the constraints in force. A node forwards it to each child it
/// lays out, presents it to a context to register a nested boundary or request a relayout, and hands it
/// to a [`DeferredLayoutScope`] to mark from a reconcile.
#[derive(Clone, Copy)]
pub struct LayoutScope(NodeId);

impl LayoutScope {
    /// A scope detached from any pipeline, which names no boundary.
    pub fn detached() -> Self {
        Self(NodeId::null())
    }

    /// Whether this scope names no boundary, because it was created detached, so registering under it
    /// would register at the root.
    pub fn is_detached(&self) -> bool {
        self.0.is_null()
    }
}

/// A [`LayoutScope`] paired with the route to mark it from outside a pipeline pass, such as a reconcile
/// that changes a layout property. The request is applied on the pipeline's next frame; a detached
/// marker, or one whose boundary is gone, marks nothing.
#[derive(Clone)]
pub struct DeferredLayoutScope {
    node_id: NodeId,
    queue: Option<Rc<RefCell<Vec<NodeId>>>>,
}

impl DeferredLayoutScope {
    /// A marker detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self {
            node_id: NodeId::null(),
            queue: None,
        }
    }

    /// Queues this boundary to be re-laid-out on the pipeline's next frame.
    pub fn mark_needs_layout(&self) {
        if let Some(queue) = &self.queue {
            queue.borrow_mut().push(self.node_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use typed_floats::{Positive, PositiveFinite};

    use crate::{
        context::{MountCtx, PaintCtx},
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        render_object::{RenderObject, box_layout::RenderBox},
        test_harness::{RawWidget, TestCtx},
        text::TextBaseline,
    };

    use super::*;

    /// The deferred handle to the boundary a [`LayoutProbe`] was last laid out under, shared back to the
    /// test.
    type Captured = Rc<RefCell<Option<DeferredLayoutScope>>>;

    /// A leaf that counts its layouts and hands the scope it was laid out under back to the test, so
    /// the test can request a relayout the way a reconcile would.
    struct LayoutProbe {
        layouts: Rc<Cell<usize>>,
        captured: Captured,
        marks_during_layout: bool,
    }

    impl RenderObject for LayoutProbe {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
    }

    impl RenderBox for LayoutProbe {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            self.layouts.set(self.layouts.get() + 1);
            *self.captured.borrow_mut() = Some(ctx.deferred_layout_scope());

            if self.marks_during_layout {
                ctx.mark_needs_layout(*ctx.scope());
            }

            constraints.smallest()
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

        fn update_compositing_bits(&mut self) -> bool {
            false
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
    }

    fn probe(marks_during_layout: bool) -> (Rc<Cell<usize>>, Captured, LayoutProbe) {
        let layouts = Rc::new(Cell::new(0));
        let captured = Rc::new(RefCell::new(None));
        let render = LayoutProbe {
            layouts: Rc::clone(&layouts),
            captured: Rc::clone(&captured),
            marks_during_layout,
        };

        (layouts, captured, render)
    }

    /// A scope is stored on every node laid out under a boundary, so it stays a single key wide, an inert
    /// id with no pointer or refcount.
    #[test]
    fn a_layout_scope_is_one_key() {
        assert_eq!(
            std::mem::size_of::<LayoutScope>(),
            std::mem::size_of::<u64>()
        );
    }

    #[test]
    fn flush_layout_skips_a_clean_frame_and_relays_out_when_marked_or_resized() {
        let (layouts, captured, render) = probe(false);

        let (mut owner, view) = TestCtx::new().mount_view(RawWidget::new(render));
        view.resize(BoxConstraints::new(0, 100, 0, 100));

        owner.flush_layout();
        assert_eq!(layouts.get(), 1, "the first frame lays the root out");

        owner.flush_layout();
        assert_eq!(layouts.get(), 1, "an unmarked frame reuses the layout");

        // The kind of out-of-band request a reconcile makes when a layout property changes.
        captured
            .borrow()
            .clone()
            .expect("laid out once")
            .mark_needs_layout();

        owner.flush_layout();
        assert_eq!(layouts.get(), 2, "a relayout request re-enters the root");

        owner.flush_layout();
        assert_eq!(layouts.get(), 2, "the request is cleared by the relayout");
    }

    #[test]
    fn a_relayout_request_schedules_a_frame_on_the_clean_to_dirty_edge() {
        let (_layouts, _captured, render) = probe(false);

        let (mut owner, view) = TestCtx::new().mount_view(RawWidget::new(render));
        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();

        // Set the hook after the seeding layout so only the marks below count.
        let frames = Rc::new(Cell::new(0));
        let scheduled = Rc::clone(&frames);
        owner.on_needs_layout(Box::new(move || scheduled.set(scheduled.get() + 1)));

        view.resize(BoxConstraints::new(0, 100, 0, 100));
        view.resize(BoxConstraints::new(0, 100, 0, 100));
        assert_eq!(
            frames.get(),
            1,
            "only the clean-to-dirty edge schedules a frame"
        );

        owner.flush_layout();
        view.resize(BoxConstraints::new(0, 100, 0, 100));
        assert_eq!(
            frames.get(),
            2,
            "a request after the flush schedules another frame"
        );
    }

    #[test]
    #[should_panic(expected = "cannot request layout while layout is in progress")]
    fn requesting_a_relayout_during_layout_panics() {
        let (_layouts, _captured, render) = probe(true);

        let (mut owner, view) = TestCtx::new().mount_view(RawWidget::new(render));
        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
    }
}
