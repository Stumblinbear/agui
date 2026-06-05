use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use fnv::FnvHashSet;
use slotmap::{SlotMap, new_key_type};

use crate::{
    constraints::Constraints,
    render_object::{
        PaintScope,
        box_layout::{AnyRenderBox, RenderBox},
    },
};

new_key_type! {
    /// Identifies one inner relayout boundary within a [`LayoutPipeline`].
    pub struct LayoutBoundaryId;
}

/// A render object shared between the layout and paint registries, so a node that is both a relayout
/// and a repaint boundary is held in one place.
pub type BoundaryContent = Rc<RefCell<dyn AnyRenderBox>>;

/// A registered relayout boundary and its depth in the boundary nesting, so the pipeline can re-enter
/// the marked boundaries rootmost-first.
struct RegisteredBoundary {
    content: BoundaryContent,
    depth: usize,

    /// The constraints this boundary was last laid out under, replayed to re-lay it on its own. A
    /// boundary constrained from outside the tree has them written by the owner.
    constraints: Option<Constraints>,

    /// The repaint boundary enclosing this one, marked when this boundary re-lays so the re-laid
    /// subtree repaints.
    paint: PaintScope,
}

/// The layout dirty state, shared so a [`LayoutScope`] can mark or register a boundary out of band.
struct LayoutPipelineState {
    dirty: FnvHashSet<LayoutBoundaryId>,
    boundaries: SlotMap<LayoutBoundaryId, RegisteredBoundary>,

    notify: Box<dyn Fn()>,

    in_layout: bool,
}

impl LayoutPipelineState {
    /// Whether nothing, root or inner, is awaiting re-layout.
    fn is_clean(&self) -> bool {
        self.dirty.is_empty()
    }

    /// Marks `boundary` for re-layout, firing the schedule hook on the clean-to-dirty edge.
    ///
    /// # Panics
    ///
    /// Panics if called during a layout pass, since the dirty set is being flushed.
    fn mark_needs_layout(&mut self, boundary: LayoutBoundaryId) {
        assert!(
            !self.in_layout,
            "a relayout cannot be requested during layout"
        );

        tracing::trace!(boundary = ?boundary, "marked boundary for re-layout");

        let was_clean = self.is_clean();
        self.dirty.insert(boundary);

        if was_clean {
            (self.notify)();
        }
    }
}

/// Lays out the relayout boundaries of one subtree, re-laying only the ones that changed.
///
/// The subtree's root is one boundary; every boundary nested inside is re-laid from the constraints it
/// last took. A change confined to a boundary re-lays just that boundary, leaving the rest untouched.
/// Mark a boundary through the [`LayoutScope`] it was given.
pub struct LayoutPipeline {
    state: Rc<RefCell<LayoutPipelineState>>,
}

fn noop() {}

impl Default for LayoutPipeline {
    fn default() -> Self {
        Self {
            state: Rc::new(RefCell::new(LayoutPipelineState {
                dirty: FnvHashSet::default(),
                boundaries: SlotMap::with_key(),

                notify: Box::new(noop),

                in_layout: false,
            })),
        }
    }
}

impl LayoutPipeline {
    /// Registers `root` as the outermost boundary, painting into `paint`, and returns the pipeline
    /// together with the scope that marks it.
    pub fn new(root: BoundaryContent, paint: PaintScope) -> (Self, LayoutScope) {
        let pipeline = Self::default();
        let scope = pipeline.insert(root, 0, paint);

        (pipeline, scope)
    }

    pub fn on_needs_layout(&mut self, f: Box<dyn Fn()>) {
        self.state.borrow_mut().notify = f;
    }

    /// Registers `content` as a boundary at `depth`, enclosed by `paint`, and returns the scope that
    /// marks it.
    fn insert(&self, content: BoundaryContent, depth: usize, paint: PaintScope) -> LayoutScope {
        let id = self
            .state
            .borrow_mut()
            .boundaries
            .insert(RegisteredBoundary {
                content,
                depth,
                constraints: None,
                paint,
            });

        LayoutScope(LayoutScopeInner::Boundary {
            state: Rc::downgrade(&self.state),
            id,
            depth,
        })
    }

    /// Re-lays every marked boundary from the constraints it last took, leaving the rest untouched.
    pub fn flush(&self) {
        // Re-enter the marked boundaries rootmost-first: re-laying an outer boundary re-lays the
        // boundaries nested in it, so doing the outer one first lets the inner ones be skipped here
        // rather than laid out a second time.
        let mut pending: Vec<LayoutBoundaryId> =
            self.state.borrow().dirty.iter().copied().collect();
        pending.sort_by_key(|id| {
            self.state
                .borrow()
                .boundaries
                .get(*id)
                .map_or(0, |boundary| boundary.depth)
        });

        self.state.borrow_mut().in_layout = true;

        for id in pending {
            let entry = {
                let mut state = self.state.borrow_mut();

                // An enclosing boundary's relayout may have already covered this one.
                if !state.dirty.remove(&id) {
                    continue;
                }

                state.boundaries.get(id).map(|boundary| {
                    (
                        Rc::clone(&boundary.content),
                        boundary.depth,
                        boundary.constraints,
                        boundary.paint.clone(),
                    )
                })
            };

            let Some((mut content, depth, Some(constraints), paint)) = entry else {
                continue;
            };

            let scope = LayoutScope(LayoutScopeInner::Boundary {
                state: Rc::downgrade(&self.state),
                id,
                depth,
            });

            content.layout(&scope, constraints);

            // The boundary's painting is now stale; repaint the boundary that encloses it.
            paint.mark_needs_paint();
        }

        self.state.borrow_mut().in_layout = false;
    }
}

/// The relayout boundary a render object is laid out under. Unlike a paint scope, which a node captures
/// once at mount, a layout scope is threaded through layout, because which node bounds a relayout is
/// decided during layout from the constraints in force. A node forwards it to each child it lays out,
/// and a node that can change its own layout out of band marks it to request a relayout. Cloning shares
/// the same target, so the scope can be marked from anywhere, including a reconcile or a per-frame
/// callback.
#[derive(Clone)]
pub struct LayoutScope(LayoutScopeInner);

#[derive(Clone)]
enum LayoutScopeInner {
    Detached,

    Boundary {
        state: Weak<RefCell<LayoutPipelineState>>,
        id: LayoutBoundaryId,
        depth: usize,
    },
}

impl LayoutScope {
    /// A scope detached from any pipeline, whose marks reach nothing.
    pub fn detached() -> Self {
        Self(LayoutScopeInner::Detached)
    }

    /// Whether this scope reaches no pipeline, so registering a boundary under it would do nothing.
    pub fn is_detached(&self) -> bool {
        matches!(self.0, LayoutScopeInner::Detached)
    }

    /// Registers `content` as a relayout boundary nested under this one and returns the scope that
    /// marks it. A boundary established during layout calls this so its descendants mark it rather than
    /// the boundary above. A detached scope registers nothing and hands back another detached scope.
    pub fn register(&self, content: BoundaryContent, paint: PaintScope) -> LayoutScope {
        let LayoutScopeInner::Boundary { state, depth, .. } = &self.0 else {
            return LayoutScope::detached();
        };

        let Some(strong) = state.upgrade() else {
            return LayoutScope::detached();
        };

        let depth = depth + 1;

        let id = strong.borrow_mut().boundaries.insert(RegisteredBoundary {
            content,
            depth,
            constraints: None,
            paint,
        });

        LayoutScope(LayoutScopeInner::Boundary {
            state: Weak::clone(state),
            id,
            depth,
        })
    }

    /// Records the constraints this boundary is re-laid under, without marking it. A boundary re-laid in
    /// place by its parent keeps its cached constraints current this way.
    pub fn update_constraints(&self, constraints: Constraints) {
        let LayoutScopeInner::Boundary { state, id, .. } = &self.0 else {
            return;
        };

        let Some(state) = state.upgrade() else {
            return;
        };

        if let Some(boundary) = state.borrow_mut().boundaries.get_mut(*id) {
            boundary.constraints = Some(constraints);
        }
    }

    /// Records the constraints this boundary is re-laid under and marks it. A boundary constrained from
    /// outside the tree is sized this way, on mount and whenever those constraints change.
    pub fn set_constraints(&self, constraints: Constraints) {
        self.update_constraints(constraints);
        self.mark_needs_layout();
    }

    /// Removes the relayout boundary this scope marks, clearing any pending re-layout of it. A root or
    /// detached scope removes nothing.
    pub fn unregister(&self) {
        let LayoutScopeInner::Boundary { state, id, .. } = &self.0 else {
            return;
        };

        let Some(state) = state.upgrade() else {
            return;
        };

        let mut state = state.borrow_mut();
        state.boundaries.remove(*id);
        state.dirty.remove(id);
    }

    /// Requests that this boundary be re-laid-out before the next frame. On the clean-to-dirty edge of
    /// the layout channel, the schedule hook fires so the driver schedules a frame. A detached scope
    /// marks nothing.
    ///
    /// # Panics
    ///
    /// Panics if called during a layout pass, since the dirty set is being flushed.
    pub fn mark_needs_layout(&self) {
        let LayoutScopeInner::Boundary { state, id, .. } = &self.0 else {
            return;
        };

        let Some(state) = state.upgrade() else {
            return;
        };

        state.borrow_mut().mark_needs_layout(*id);
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
        constraints::Constraints,
        hit_test::{HitTest, HitTestResult},
        offset::Offset,
        paint::{ContainerLayer, LayerHandle, PaintCtx},
        render_object::{MountCtx, PipelineOwner, RenderObject, box_layout::RenderBox},
        size::Size,
        text_baseline::TextBaseline,
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
        fn measure(&self, constraints: Constraints) -> Size {
            constraints.smallest()
        }
        fn layout(&mut self, scope: &LayoutScope, constraints: Constraints) -> Size {
            self.layouts.set(self.layouts.get() + 1);
            *self.captured.borrow_mut() = Some(scope.clone());

            if self.marks_during_layout {
                scope.mark_needs_layout();
            }

            constraints.smallest()
        }
        fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
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

    #[test]
    fn flush_layout_skips_a_clean_frame_and_relays_out_when_marked_or_resized() {
        let (layouts, captured, render) = probe(false);

        let mut owner = PipelineOwner::new(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(ContainerLayer::new()),
        );
        owner.resize(Constraints::new(0, 100, 0, 100));

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

        let mut owner = PipelineOwner::new(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(ContainerLayer::new()),
        );
        owner.resize(Constraints::new(0, 100, 0, 100));
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
    #[should_panic(expected = "a relayout cannot be requested during layout")]
    fn requesting_a_relayout_during_layout_panics() {
        let (_layouts, _captured, render) = probe(true);

        let mut owner = PipelineOwner::new(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(ContainerLayer::new()),
        );
        owner.resize(Constraints::new(0, 100, 0, 100));
        owner.flush_layout();
    }
}
