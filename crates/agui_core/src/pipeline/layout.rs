// The intrusive-collections adapter macro emits a manual `Clone` on a zero-sized `Copy` adapter.
#![allow(clippy::expl_impl_clone_on_copy)]

use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use intrusive_collections::{LinkedList, LinkedListLink, UnsafeRef, intrusive_adapter};

use crate::{
    context::LayoutCtx,
    pipeline::{
        BoundaryContent,
        paint::{PaintPipeline, PaintScope},
    },
    render_object::box_layout::{AnyRenderBox, BoxConstraints, RenderBox},
};

/// Lays out the relayout boundaries of one subtree, re-laying only the ones that changed.
///
/// The subtree's root is one boundary; every boundary nested inside is re-laid from the constraints it
/// last took. A change confined to a boundary re-lays just that boundary, leaving the rest untouched.
/// Mark a boundary through the [`LayoutScope`] it was given.
pub struct LayoutPipeline {
    state: Rc<RefCell<LayoutPipelineState>>,
}

fn noop() {}

impl Drop for LayoutPipeline {
    fn drop(&mut self) {
        // The dirty list holds non-owning references into cells the registry is about to free; empty it
        // first so no link outlives its cell.
        self.state.borrow_mut().dirty.fast_clear();
    }
}

impl Default for LayoutPipeline {
    fn default() -> Self {
        Self {
            state: Rc::new(RefCell::new(LayoutPipelineState {
                dirty: LinkedList::new(LayoutCellAdapter::new()),

                notify: Box::new(noop),

                in_layout: false,
            })),
        }
    }
}

impl LayoutPipeline {
    pub fn on_needs_layout(&mut self, f: Box<dyn Fn()>) {
        self.state.borrow_mut().notify = f;
    }

    /// Registers `root` as the outermost boundary, enclosed by `paint`, returning the handle that owns
    /// and marks it.
    pub fn register_root(
        &self,
        root: BoundaryContent,
        paint: PaintScope,
    ) -> RegisteredLayoutBoundary {
        self.insert(root, 0, paint)
    }

    /// Registers `content` as a boundary at `depth`, enclosed by `paint`, and returns the handle that
    /// owns and marks it.
    fn insert(
        &self,
        content: BoundaryContent,
        depth: usize,
        paint: PaintScope,
    ) -> RegisteredLayoutBoundary {
        let cell = Rc::new(LayoutCell {
            link: LinkedListLink::new(),
            state: Rc::downgrade(&self.state),
            depth,
            content,
            constraints: Cell::new(None),
            paint,
            is_dirty: Cell::new(false),
            needs_layout: Cell::new(false),
        });

        RegisteredLayoutBoundary { cell }
    }

    /// Re-lays every marked boundary from the constraints it last took, leaving the rest untouched.
    pub fn flush(&self, paint_pipeline: &mut PaintPipeline) {
        self.state.borrow_mut().in_layout = true;

        // Rootmost-first so that re-laying an outer boundary, which re-lays the boundaries nested in it,
        // lets the inner ones be skipped here rather than laid out a second time.
        let ordered = self.drain_rootmost_first();

        for cell in ordered {
            // An enclosing boundary's relayout may have already covered this one: re-laying it in
            // place cleared its pending mark, and unregistering it dropped the mark with the handle.
            if !cell.needs_layout.replace(false) {
                continue;
            }

            let Some(constraints) = cell.constraints.get() else {
                continue;
            };

            let scope = LayoutScope(Rc::downgrade(&cell));

            let mut ctx = LayoutCtx::new(self, &mut *paint_pipeline, scope);

            let mut content = Rc::clone(&cell.content);
            content.layout(&mut ctx, constraints);

            // The boundary's painting is now stale; repaint the boundary that encloses it.
            cell.paint.mark_needs_paint();
        }

        self.state.borrow_mut().in_layout = false;
    }

    /// Drains the dirty list into the owning `Rc`s, ordered shallowest-depth first.
    ///
    /// The single-cell frame returns one cell directly, with no allocation or sort. The owning `Rc`s
    /// are held for the rest of the flush so a boundary that goes loose mid-pass, dropping its owner
    /// handle, cannot free a cell the flush still needs.
    fn drain_rootmost_first(&self) -> Vec<Rc<LayoutCell>> {
        let mut state = self.state.borrow_mut();

        let Some(first) = state.dirty.pop_front() else {
            return Vec::new();
        };

        first.is_dirty.set(false);
        let first = recover_owner(first);

        if state.dirty.is_empty() {
            return vec![first];
        }

        let mut ordered = vec![first];
        while let Some(unlinked) = state.dirty.pop_front() {
            unlinked.is_dirty.set(false);
            ordered.push(recover_owner(unlinked));
        }

        ordered.sort_unstable_by_key(|cell| cell.depth);

        ordered
    }
}

/// Recovers a counted owning `Rc` from a non-owning ref popped off the dirty list.
fn recover_owner(popped: UnsafeRef<LayoutCell>) -> Rc<LayoutCell> {
    let ptr = UnsafeRef::into_raw(popped);

    // SAFETY: the ref was created with `UnsafeRef::from_raw(Rc::as_ptr(&cell))`, so `ptr` carries the
    // whole-allocation provenance of a live `Rc<LayoutCell>` whose owner outlives this call. Bumping the
    // strong count before reconstructing balances the `Rc` this produces against that still-live owner.
    unsafe {
        Rc::increment_strong_count(ptr);
        Rc::from_raw(ptr)
    }
}

/// A registered relayout boundary, owned by its [`RegisteredLayoutBoundary`] handle and linked into the
/// dirty list while it awaits re-layout.
struct LayoutCell {
    link: LinkedListLink,

    /// The pipeline state this cell's marks are linked into.
    state: Weak<RefCell<LayoutPipelineState>>,

    /// The depth in the boundary nesting, sorted at flush so the pipeline re-enters marked boundaries
    /// rootmost-first.
    depth: usize,

    content: BoundaryContent,

    /// The constraints this boundary was last laid out under, replayed to re-lay it on its own. A
    /// boundary constrained from outside the tree has them written by the owner.
    constraints: Cell<Option<BoxConstraints>>,

    /// The repaint boundary enclosing this one, marked when this boundary re-lays so the re-laid
    /// subtree repaints.
    paint: PaintScope,

    /// Whether this cell is currently linked into the dirty list, guarding a double-mark from linking it
    /// twice.
    is_dirty: Cell<bool>,

    /// Whether this boundary is awaiting re-layout. Outlives the cell's place in the dirty list, and
    /// clears when the boundary is re-laid, whether by the flush or in place by the boundary above it.
    needs_layout: Cell<bool>,
}

intrusive_adapter!(LayoutCellAdapter = UnsafeRef<LayoutCell>: LayoutCell { link => LinkedListLink });

/// The layout dirty state, shared so a [`LayoutScope`] can mark a boundary out of band.
struct LayoutPipelineState {
    dirty: LinkedList<LayoutCellAdapter>,

    notify: Box<dyn Fn()>,

    in_layout: bool,
}

impl LayoutPipelineState {
    /// Whether nothing, root or inner, is awaiting re-layout.
    fn is_clean(&self) -> bool {
        self.dirty.is_empty()
    }

    /// Marks `cell` for re-layout, firing the schedule hook on the clean-to-dirty edge.
    ///
    /// # Panics
    ///
    /// Panics if called during a layout pass, since the dirty list is being flushed.
    fn mark_needs_layout(&mut self, cell: &Rc<LayoutCell>) {
        assert!(
            !self.in_layout,
            "cannot request layout while layout is in progress"
        );

        cell.needs_layout.set(true);

        if cell.is_dirty.get() {
            return;
        }

        tracing::trace!(boundary = ?Rc::as_ptr(cell), "marked boundary for re-layout");

        let was_clean = self.is_clean();

        cell.is_dirty.set(true);
        // SAFETY: the cell is owned by its `RegisteredLayoutBoundary` handle for as long as it is
        // registered, and that handle unlinks it before the `Rc` is dropped; the dirty flag guards it
        // against being linked into the list more than once.
        self.dirty
            .push_back(unsafe { UnsafeRef::from_raw(Rc::as_ptr(cell)) });

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
    cell: Rc<LayoutCell>,
}

impl RegisteredLayoutBoundary {
    /// A clone-able scope that marks this boundary and registers descendants under it.
    pub fn scope(&self) -> LayoutScope {
        LayoutScope(Rc::downgrade(&self.cell))
    }

    /// Records the constraints this boundary is re-laid under, without marking it. A boundary re-laid in
    /// place by its parent keeps its cached constraints current this way, and that relayout satisfies
    /// any re-layout still pending on the boundary.
    pub fn update_constraints(&self, constraints: BoxConstraints) {
        self.cell.constraints.set(Some(constraints));
        self.cell.needs_layout.set(false);
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
    /// Panics if called during a layout pass, since the dirty list is being flushed.
    pub fn mark_needs_layout(&self) {
        if let Some(state) = self.cell.state.upgrade() {
            state.borrow_mut().mark_needs_layout(&self.cell);
        }
    }
}

impl Drop for RegisteredLayoutBoundary {
    fn drop(&mut self) {
        // An unregistered boundary can no longer be re-laid on its own; a flush that already drained
        // its cell must skip it rather than replay its stale constraints.
        self.cell.needs_layout.set(false);

        if !self.cell.is_dirty.get() {
            return;
        }

        let Some(state) = self.cell.state.upgrade() else {
            return;
        };

        let mut state = state.borrow_mut();

        // SAFETY: the cell is linked into this list and stays live behind its `Rc` until this handle's
        // `Rc` frees it after this method returns, so the pointer the cursor recovers is valid.
        let mut cursor = unsafe { state.dirty.cursor_mut_from_ptr(Rc::as_ptr(&self.cell)) };
        cursor.remove();
        self.cell.is_dirty.set(false);
    }
}

/// The relayout boundary a render object is laid out under. Unlike a paint scope, which a node captures
/// once at mount, a layout scope is threaded through layout, because which node bounds a relayout is
/// decided during layout from the constraints in force. A node forwards it to each child it lays out,
/// and a node that can change its own layout out of band marks it to request a relayout. Cloning shares
/// the same target, so the scope can be marked from anywhere, including a reconcile or a per-frame
/// callback.
#[derive(Clone)]
pub struct LayoutScope(Weak<LayoutCell>);

impl LayoutScope {
    /// A scope detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self(Weak::new())
    }

    /// Whether this scope reaches no live boundary, because it was created detached or its boundary
    /// has since been freed, so registering under it would do nothing.
    pub fn is_detached(&self) -> bool {
        Weak::strong_count(&self.0) == 0
    }

    /// Registers `content` as a relayout boundary nested under this one and returns the handle that owns
    /// and marks it. A boundary established during layout calls this so its descendants mark it rather
    /// than the boundary above. A detached scope registers nothing and hands back a handle whose marks
    /// reach nothing.
    pub fn register(
        &self,
        content: BoundaryContent,
        paint: PaintScope,
    ) -> RegisteredLayoutBoundary {
        let (state, depth) = match self.0.upgrade() {
            Some(enclosing) => (Weak::clone(&enclosing.state), enclosing.depth + 1),
            // A detached handle reaches no pipeline, so its cell is never marked or linked.
            None => (Weak::new(), 0),
        };

        let cell = Rc::new(LayoutCell {
            link: LinkedListLink::new(),
            state,
            depth,
            content,
            constraints: Cell::new(None),
            paint,
            is_dirty: Cell::new(false),
            needs_layout: Cell::new(false),
        });

        RegisteredLayoutBoundary { cell }
    }

    /// Requests that this boundary be re-laid-out before the next frame. On the clean-to-dirty edge of
    /// the layout channel, the schedule hook fires so the driver schedules a frame. A detached scope
    /// marks nothing.
    ///
    /// # Panics
    ///
    /// Panics if called during a layout pass, since the dirty list is being flushed.
    pub fn mark_needs_layout(&self) {
        let Some(cell) = self.0.upgrade() else {
            return;
        };

        let Some(state) = cell.state.upgrade() else {
            return;
        };

        state.borrow_mut().mark_needs_layout(&cell);
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
        paint::compositing::{LayerHandle, OffsetLayer},
        pipeline::PipelineOwner,
        render_object::{RenderObject, box_layout::RenderBox},
        test_harness::{RawWidget, mount_view},
        text::TextBaseline,
    };

    use super::*;

    /// The scope a [`LayoutProbe`] was last laid out under, shared back to the test.
    type Captured = Rc<RefCell<Option<LayoutScope>>>;

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
        fn update_compositing_bits(&mut self) -> bool {
            false
        }
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
            *self.captured.borrow_mut() = Some(ctx.scope().clone());

            if self.marks_during_layout {
                ctx.scope().mark_needs_layout();
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

    /// A scope is cloned onto every node that can mark its boundary, so it stays one pointer wide.
    #[test]
    fn a_layout_scope_is_one_pointer() {
        assert_eq!(
            std::mem::size_of::<LayoutScope>(),
            std::mem::size_of::<usize>()
        );
    }

    #[test]
    fn flush_layout_skips_a_clean_frame_and_relays_out_when_marked_or_resized() {
        let (layouts, captured, render) = probe(false);

        let mut owner = PipelineOwner::from_root(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(OffsetLayer::new()),
        );
        owner.resize(BoxConstraints::new(0, 100, 0, 100));

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
        let (_layouts, captured, render) = probe(false);

        let mut owner = PipelineOwner::from_root(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(OffsetLayer::new()),
        );
        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        let captured = captured.borrow().clone().expect("laid out once");

        // Set the hook after the seeding layout so only the marks below count.
        let frames = Rc::new(Cell::new(0));
        let scheduled = Rc::clone(&frames);
        owner.on_needs_layout(Box::new(move || scheduled.set(scheduled.get() + 1)));

        captured.mark_needs_layout();
        captured.mark_needs_layout();
        assert_eq!(
            frames.get(),
            1,
            "only the clean-to-dirty edge schedules a frame"
        );

        owner.flush_layout();
        captured.mark_needs_layout();
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

        let mut owner = PipelineOwner::from_root(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(OffsetLayer::new()),
        );
        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
    }
}
