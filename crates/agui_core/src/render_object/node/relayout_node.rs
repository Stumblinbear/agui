use std::{cell::RefCell, hint::unreachable_unchecked, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::PaintCtx,
    diagnostics::{Diagnostics, DiagnosticsNode},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    pipeline::{
        BoundaryContent,
        layout::{LayoutScope, RegisteredLayoutBoundary},
        paint::PaintScope,
    },
    render_object::{
        LayoutCtx, MountCtx,
        box_layout::{BoxConstraints, RenderBox},
    },
    text::TextBaseline,
};

/// A child holder whose child becomes a relayout boundary while it is constrained tightly.
///
/// A render object holds a child here instead of in a [`RenderNode`](super::RenderNode) when the child
/// can be laid out on its own: while the constraints handed down are tight, the child's size is fixed by
/// them, so a change confined to its subtree re-lays only the child, not the holder or anything above.
/// The child is monomorphized and free of overhead until the first tight layout, when it is shared and
/// registered as a boundary; it returns to that free-of-overhead form once it has been loosely
/// constrained for a sustained run.
pub struct RelayoutRenderNode<R, P = ()> {
    pub parent_data: P,

    child: RelayoutChild<R>,

    /// Consecutive layouts under loose constraints, reset to zero by any tight layout.
    loose_streak: u8,

    /// The enclosing repaint boundary, captured at mount; detached until then.
    paint: PaintScope,
    parent_uses_size: bool,
    needs_compositing: bool,
}

enum RelayoutChild<R> {
    /// The child held by value, while it is not currently a relayout boundary.
    Inline(R),

    /// The child shared behind an `Rc<RefCell>`, once it has been a relayout boundary. `boundary` owns
    /// the registration while it is registered, dropped while loose constraints make it unsound to
    /// re-lay alone. The child returns to [`Inline`](Self::Inline) once it has been loosely constrained
    /// for long enough.
    Boxed {
        content: Rc<RefCell<R>>,
        boundary: Option<RegisteredLayoutBoundary>,
    },
}

enum Form {
    BoxAndRegister,
    Register,
    Unregister,
    Unbox,
    Keep,
}

impl<R, P: Default> RelayoutRenderNode<R, P> {
    pub fn new(child: R) -> Self {
        Self {
            parent_data: P::default(),

            child: RelayoutChild::Inline(child),

            loose_streak: 0,

            paint: PaintScope::detached(),
            parent_uses_size: false,
            needs_compositing: false,
        }
    }
}

impl<R: RenderBox, P> RelayoutRenderNode<R, P> {
    pub fn mount(&mut self, ctx: &mut MountCtx) {
        self.paint = ctx.paint_scope().clone();

        match &mut self.child {
            RelayoutChild::Inline(child) => child.mount(ctx),
            RelayoutChild::Boxed { content, .. } => content.borrow_mut().mount(ctx),
        }
    }

    pub fn unmount(&mut self, ctx: &mut MountCtx) {
        match &mut self.child {
            RelayoutChild::Inline(child) => child.unmount(ctx),
            RelayoutChild::Boxed { content, boundary } => {
                content.borrow_mut().unmount(ctx);

                // Dropping the handle unregisters the boundary and unlinks it from the dirty list.
                boundary.take();
            }
        }
    }

    pub fn update_compositing_bits(&mut self) -> bool {
        self.needs_compositing = match &mut self.child {
            RelayoutChild::Inline(child) => child.update_compositing_bits(),
            RelayoutChild::Boxed { content, .. } => content.borrow_mut().update_compositing_bits(),
        };

        self.needs_compositing
    }

    /// Whether this child's subtree contributes a compositing layer, as of the last recompute.
    pub fn needs_compositing(&self) -> bool {
        self.needs_compositing
    }

    /// Captures the held render object's subtree, annotated with this holder's pipeline state.
    pub fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        let dec = d
            .decorate()
            .flag("parent_uses_size", self.parent_uses_size)
            .flag("needs_compositing", self.needs_compositing);

        match &self.child {
            RelayoutChild::Inline(child) => dec.child(|d| child.describe(d)),

            RelayoutChild::Boxed { content, boundary } => dec
                .property(
                    "relayout_boundary",
                    if boundary.is_some() {
                        "registered"
                    } else {
                        "unregistered"
                    },
                )
                .child(|d| content.borrow().describe(d)),
        }
    }

    pub fn with_object<T>(&self, f: impl FnOnce(&R) -> T) -> T {
        match &self.child {
            RelayoutChild::Inline(child) => f(child),
            RelayoutChild::Boxed { content, .. } => f(&content.borrow()),
        }
    }

    /// Reconciles the child, reaching it through the shared cell while it is a boundary.
    pub fn with_object_mut<T>(&mut self, f: impl FnOnce(&mut R) -> T) -> T {
        match &mut self.child {
            RelayoutChild::Inline(child) => f(child),
            RelayoutChild::Boxed { content, .. } => f(&mut content.borrow_mut()),
        }
    }

    #[cfg(test)]
    fn is_inline(&self) -> bool {
        matches!(self.child, RelayoutChild::Inline(_))
    }

    #[cfg(test)]
    fn is_registered(&self) -> bool {
        matches!(
            self.child,
            RelayoutChild::Boxed {
                boundary: Some(_),
                ..
            }
        )
    }

    pub fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.min_intrinsic_width(height),
            RelayoutChild::Boxed { content, .. } => content.borrow().min_intrinsic_width(height),
        }
    }

    pub fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.max_intrinsic_width(height),
            RelayoutChild::Boxed { content, .. } => content.borrow().max_intrinsic_width(height),
        }
    }

    pub fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.min_intrinsic_height(width),
            RelayoutChild::Boxed { content, .. } => content.borrow().min_intrinsic_height(width),
        }
    }

    pub fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.max_intrinsic_height(width),
            RelayoutChild::Boxed { content, .. } => content.borrow().max_intrinsic_height(width),
        }
    }

    pub fn measure(&self, constraints: BoxConstraints) -> Size {
        match &self.child {
            RelayoutChild::Inline(child) => child.measure(constraints),
            RelayoutChild::Boxed { content, .. } => content.borrow().measure(constraints),
        }
    }

    /// Lays this child out under `constraints`. If you need the resulting size, use
    /// `layout_and_get_size` instead.
    pub fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) {
        self.layout_inner(ctx, constraints);
    }

    /// Lays this child out under `constraints` and returns the size it took. This couples the holder with
    /// the child, so a change to the child's size re-lays the holder too.
    pub fn layout_and_get_size(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
    ) -> Size {
        self.parent_uses_size = true;

        self.layout_inner(ctx, constraints)
    }

    fn layout_inner(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.reshape(constraints.is_tight(), ctx.scope());

        match &mut self.child {
            RelayoutChild::Inline(child) => child.layout(ctx, constraints),

            RelayoutChild::Boxed { content, boundary } => match boundary {
                Some(reg) => {
                    reg.update_constraints(constraints);

                    ctx.with_layout_scope(reg.scope(), |ctx| {
                        content.borrow_mut().layout(ctx, constraints)
                    })
                }

                None => content.borrow_mut().layout(ctx, constraints),
            },
        }
    }

    /// Loose layouts a registered boundary must survive in a row before it is recovered to the inline
    /// form.
    // An unbox costs ~75 ns and a re-box ~83 ns, while a boxed-but-unregistered child costs only a few
    // ns more per layout and paint call than an inline one, so recovery only pays off after many loose
    // layouts. The threshold is set high so a node that merely flickers loose for a frame never pays the
    // round trip; only a node that has genuinely settled loose recovers. Any value >= 1 is correct,
    // since the boundary is unregistered before the unbox either way; this is purely a perf heuristic.
    const UNBOX_AFTER_LOOSE_LAYOUTS: u8 = 32;

    /// Moves the child between its inline and boundary forms to match whether it is now constrained
    /// tightly, recovering it to the inline form only after it has stayed loosely constrained for
    /// [`UNBOX_AFTER_LOOSE_LAYOUTS`](Self::UNBOX_AFTER_LOOSE_LAYOUTS) layouts in a row.
    fn reshape(&mut self, tight: bool, scope: &LayoutScope) {
        // A detached scope cannot register a boundary, so the child stays inline; this also keeps an
        // unmounted layout (a measurement or a test) from needing a paint scope it has not captured.
        let registrable = tight && !scope.is_detached();

        if tight {
            self.loose_streak = 0;
        } else {
            self.loose_streak = self.loose_streak.saturating_add(1);
        }

        let form = match &self.child {
            RelayoutChild::Inline(_) if registrable => Form::BoxAndRegister,

            RelayoutChild::Boxed { boundary: None, .. } if registrable => Form::Register,

            RelayoutChild::Boxed {
                boundary: Some(_), ..
            } if !tight => Form::Unregister,

            RelayoutChild::Boxed { boundary: None, .. }
                if self.loose_streak >= Self::UNBOX_AFTER_LOOSE_LAYOUTS =>
            {
                Form::Unbox
            }

            _ => Form::Keep,
        };

        match form {
            Form::BoxAndRegister => {
                debug_assert!(
                    !self.paint.is_detached(),
                    "child must be mounted before it is laid out"
                );

                let paint = self.paint.clone();

                take(&mut self.child, |child| {
                    let RelayoutChild::Inline(child) = child else {
                        // SAFETY: The inline form was just observed, so it must be Inline.
                        unsafe {
                            unreachable_unchecked();
                        }
                    };

                    let content = Rc::new(RefCell::new(child));
                    let boundary = scope.register(erase(Rc::clone(&content)), paint);

                    RelayoutChild::Boxed {
                        content,
                        boundary: Some(boundary),
                    }
                });
            }

            Form::Register => {
                debug_assert!(
                    !self.paint.is_detached(),
                    "child must be mounted before it is laid out"
                );

                let paint = self.paint.clone();

                if let RelayoutChild::Boxed { content, boundary } = &mut self.child {
                    *boundary = Some(scope.register(erase(Rc::clone(content)), paint));
                }
            }

            Form::Unregister => {
                if let RelayoutChild::Boxed { boundary, .. } = &mut self.child {
                    // Dropping the handle unregisters the boundary and unlinks it from the dirty list.
                    boundary.take();
                }
            }

            Form::Unbox => self.unbox(),

            Form::Keep => {}
        }
    }

    /// Returns a settled-loose child from its boundary form to the inline, statically dispatched one. If
    /// something other than this holder still shares the child, it stays boxed but no longer registered
    /// and is retried on the next loose layout.
    fn unbox(&mut self) {
        if let RelayoutChild::Boxed { boundary, .. } = &mut self.child {
            // Dropping the handle unregisters the boundary and unlinks it from the dirty list.
            boundary.take();
        }

        take(&mut self.child, |child| {
            let RelayoutChild::Boxed { content, boundary } = child else {
                // SAFETY: The boxed form was just observed, so it must be Boxed.
                unsafe {
                    unreachable_unchecked();
                }
            };

            // The boundary handle and its content clone were just dropped, so this holder is the sole
            // owner; try_unwrap fails only if a reconcile borrow still outlives this layout.
            match Rc::try_unwrap(content) {
                Ok(cell) => RelayoutChild::Inline(cell.into_inner()),
                Err(content) => RelayoutChild::Boxed { content, boundary },
            }
        });
    }

    pub fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.measure_baseline(constraints, baseline),
            RelayoutChild::Boxed { content, .. } => {
                content.borrow().measure_baseline(constraints, baseline)
            }
        }
    }

    pub fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        match &mut self.child {
            RelayoutChild::Inline(child) => child.distance_to_baseline(baseline),
            RelayoutChild::Boxed { content, .. } => {
                content.borrow_mut().distance_to_baseline(baseline)
            }
        }
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        match &self.child {
            RelayoutChild::Inline(child) => child.hit_test(result, position),
            RelayoutChild::Boxed { content, .. } => content.borrow().hit_test(result, position),
        }
    }

    pub fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        match &mut self.child {
            RelayoutChild::Inline(child) => child.paint(ctx, offset),
            RelayoutChild::Boxed { content, .. } => content.borrow_mut().paint(ctx, offset),
        }
    }
}

/// Erases a shared child into the form the boundary registries hold.
fn erase<R: RenderBox>(content: Rc<RefCell<R>>) -> BoundaryContent {
    content
}

/// Replaces the value behind `slot` with `f` applied to it, passing the old value through by value.
///
/// Aborts the process if `f` panics, since the old value is moved out of `slot` for the duration of the
/// call and unwinding past that point would drop it a second time.
fn take<T>(slot: &mut T, f: impl FnOnce(T) -> T) {
    struct AbortOnUnwind;

    impl Drop for AbortOnUnwind {
        fn drop(&mut self) {
            std::process::abort();
        }
    }

    // SAFETY: `read` moves the value out of `slot` without dropping the bits left behind, and `write`
    // overwrites those bits with `f`'s result without dropping them, so the value is moved exactly once.
    // `f` only touches the value it is given, never `slot`, so nothing observes the gap; the guard turns a
    // panic in `f` into an abort before the duplicated value behind `slot` could be dropped twice.
    unsafe {
        let old = std::ptr::read(slot);
        let guard = AbortOnUnwind;
        let new = f(old);
        std::mem::forget(guard);
        std::ptr::write(slot, new);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use typed_floats::{Positive, PositiveFinite, as_const};

    use crate::{
        context::PaintCtx,
        diagnostics::{Diagnostics, DiagnosticsNode},
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        pipeline::PipelineOwner,
        render_object::{
            MountCtx, RenderObject,
            box_layout::{BoxConstraints, RenderBox},
        },
        test_harness::{RawWidget, TestCtx},
        text::TextBaseline,
        view::ViewHandle,
    };

    use super::*;

    type Captured = Rc<RefCell<Option<LayoutScope>>>;

    /// A leaf that counts its layouts and paints and captures the scope and constraints it was laid
    /// out under, so a test can re-lay it the way a change inside it would.
    struct Probe {
        layouts: Rc<Cell<usize>>,
        paints: Rc<Cell<usize>>,
        captured: Captured,
        constraints_seen: Rc<Cell<Option<BoxConstraints>>>,
    }

    impl RenderObject for Probe {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for Probe {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            self.layouts.set(self.layouts.get() + 1);
            *self.captured.borrow_mut() = Some(ctx.scope().clone());
            self.constraints_seen.set(Some(constraints));

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

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {
            self.paints.set(self.paints.get() + 1);
        }
    }

    /// A holder that lays its child out tightly, so the child becomes a relayout boundary, and counts
    /// its own layouts so a test can confirm it is not re-laid when only the child changes.
    struct Tighten {
        layouts: Rc<Cell<usize>>,
        child: RelayoutRenderNode<Probe, Option<Size>>,
    }

    impl RenderObject for Tighten {
        fn mount(&mut self, ctx: &mut MountCtx) {
            self.child.mount(ctx);
        }

        fn unmount(&mut self, ctx: &mut MountCtx) {
            self.child.unmount(ctx);
        }

        fn update_compositing_bits(&mut self) -> bool {
            self.child.update_compositing_bits()
        }

        fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
            d.node_for::<Self>()
                .child(|d| self.child.describe(d))
                .finish()
        }
    }

    impl RenderBox for Tighten {
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

        fn measure(&self, _: BoxConstraints) -> Size {
            Size::new(10.0, 10.0)
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, _: BoxConstraints) -> Size {
            self.layouts.set(self.layouts.get() + 1);

            let size = Size::new(10.0, 10.0);
            self.child.layout(ctx, BoxConstraints::tight(size));

            size
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

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.child.paint(ctx, offset);
        }
    }

    #[test]
    fn a_tight_child_re_lays_and_repaints_on_its_own() {
        let probe_layouts = Rc::new(Cell::new(0));
        let probe_paints = Rc::new(Cell::new(0));
        let captured: Captured = Rc::new(RefCell::new(None));
        let tighten_layouts = Rc::new(Cell::new(0));

        let tighten = Tighten {
            layouts: Rc::clone(&tighten_layouts),
            child: RelayoutRenderNode::new(Probe {
                layouts: Rc::clone(&probe_layouts),
                paints: Rc::clone(&probe_paints),
                captured: Rc::clone(&captured),
                constraints_seen: Rc::new(Cell::new(None)),
            }),
        };

        let (mut owner, view) = TestCtx::new().mount_view(RawWidget::new(tighten));

        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        owner.flush_paint();
        assert_eq!(probe_layouts.get(), 1);
        assert_eq!(probe_paints.get(), 1);
        assert_eq!(tighten_layouts.get(), 1);

        // The boundary the probe captured is the one the tight holder registered, not the root.
        captured
            .borrow()
            .clone()
            .expect("laid out once")
            .mark_needs_layout();

        owner.flush_layout();
        assert_eq!(probe_layouts.get(), 2, "the tight child re-laid on its own");
        assert_eq!(
            tighten_layouts.get(),
            1,
            "the holder above the boundary was not re-laid"
        );

        owner.flush_paint();
        assert_eq!(
            probe_paints.get(),
            2,
            "the relayout repainted the boundary enclosing the child"
        );
    }

    #[test]
    fn an_outer_relayout_covers_a_dirty_inner_boundary() {
        let probe_layouts = Rc::new(Cell::new(0));
        let captured: Captured = Rc::new(RefCell::new(None));
        let tighten_layouts = Rc::new(Cell::new(0));

        let tighten = Tighten {
            layouts: Rc::clone(&tighten_layouts),
            child: RelayoutRenderNode::new(Probe {
                layouts: Rc::clone(&probe_layouts),
                paints: Rc::new(Cell::new(0)),
                captured: Rc::clone(&captured),
                constraints_seen: Rc::new(Cell::new(None)),
            }),
        };

        let (mut owner, view) = TestCtx::new().mount_view(RawWidget::new(tighten));

        view.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        assert_eq!(probe_layouts.get(), 1);
        assert_eq!(tighten_layouts.get(), 1);

        // Mark the inner boundary and the root in the same frame; the root's relayout re-lays the
        // inner child in place, which satisfies the inner mark.
        captured
            .borrow()
            .clone()
            .expect("laid out once")
            .mark_needs_layout();
        view.resize(BoxConstraints::new(0, 200, 0, 200));

        owner.flush_layout();
        assert_eq!(tighten_layouts.get(), 2, "the root re-laid");
        assert_eq!(
            probe_layouts.get(),
            2,
            "the inner boundary covered by the outer relayout is not re-laid a second time"
        );
    }

    /// A holder that lays its child out tightly or loosely depending on a flag, and reports the child's
    /// form after each layout, so a test can watch it box, unregister, and recover as the flag changes.
    struct Toggle {
        tight: Rc<Cell<bool>>,
        inline_after: Rc<Cell<bool>>,
        registered_after: Rc<Cell<bool>>,
        child: RelayoutRenderNode<Probe, Option<Size>>,
    }

    impl RenderObject for Toggle {
        fn mount(&mut self, ctx: &mut MountCtx) {
            self.child.mount(ctx);
        }
        fn unmount(&mut self, ctx: &mut MountCtx) {
            self.child.unmount(ctx);
        }
        fn update_compositing_bits(&mut self) -> bool {
            self.child.update_compositing_bits()
        }
    }

    impl RenderBox for Toggle {
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
        fn measure(&self, _: BoxConstraints) -> Size {
            Size::new(10.0, 10.0)
        }
        fn layout(&mut self, ctx: &mut LayoutCtx, _: BoxConstraints) -> Size {
            let size = Size::new(10.0, 10.0);

            let constraints = if self.tight.get() {
                BoxConstraints::tight(size)
            } else {
                BoxConstraints::new(0, 10, 0, 10)
            };
            self.child.layout(ctx, constraints);

            self.inline_after.set(self.child.is_inline());
            self.registered_after.set(self.child.is_registered());

            size
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
        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.child.paint(ctx, offset);
        }
    }

    /// A mounted [`Toggle`] root and the handles a test uses to flip its constraints and read the child's
    /// form after each layout.
    struct Driver {
        tight: Rc<Cell<bool>>,
        inline_after: Rc<Cell<bool>>,
        registered_after: Rc<Cell<bool>>,
        probe_captured: Captured,
        probe_constraints: Rc<Cell<Option<BoxConstraints>>>,
        owner: PipelineOwner,
        view: ViewHandle,
    }

    impl Driver {
        fn new() -> Self {
            let tight = Rc::new(Cell::new(true));
            let inline_after = Rc::new(Cell::new(false));
            let registered_after = Rc::new(Cell::new(false));
            let probe_captured: Captured = Rc::new(RefCell::new(None));
            let probe_constraints = Rc::new(Cell::new(None));

            let toggle = Toggle {
                tight: Rc::clone(&tight),
                inline_after: Rc::clone(&inline_after),
                registered_after: Rc::clone(&registered_after),
                child: RelayoutRenderNode::new(Probe {
                    layouts: Rc::new(Cell::new(0)),
                    paints: Rc::new(Cell::new(0)),
                    captured: Rc::clone(&probe_captured),
                    constraints_seen: Rc::clone(&probe_constraints),
                }),
            };

            let (owner, view) = TestCtx::new().mount_view(RawWidget::new(toggle));

            Self {
                tight,
                inline_after,
                registered_after,
                probe_captured,
                probe_constraints,
                owner,
                view,
            }
        }

        /// Re-lays the root, so the toggle re-lays its child under the constraints its flag selects.
        fn relay(&mut self) {
            self.view.resize(BoxConstraints::new(0, 100, 0, 100));
            self.owner.flush_layout();
        }
    }

    #[test]
    fn a_settled_loose_child_recovers_to_the_inline_form() {
        let k = u32::from(RelayoutRenderNode::<Probe, Option<Size>>::UNBOX_AFTER_LOOSE_LAYOUTS);

        let mut d = Driver::new();

        // One tight layout boxes and registers the child as a boundary.
        d.relay();
        assert!(
            !d.inline_after.get() && d.registered_after.get(),
            "a tight child is boxed and registered"
        );

        // The first loose layout unregisters it but keeps it boxed, well short of the threshold.
        d.tight.set(false);
        d.relay();
        assert!(
            !d.inline_after.get() && !d.registered_after.get(),
            "the first loose layout unregisters but stays boxed"
        );

        // Loose layouts up to the threshold leave it boxed; the one at the threshold recovers it.
        for layout in 2..k {
            d.relay();
            assert!(
                !d.inline_after.get(),
                "still boxed after {layout} loose layouts, below the threshold"
            );
        }

        d.relay();
        assert!(
            d.inline_after.get(),
            "a child loose for the threshold run of layouts recovers to the inline form"
        );
    }

    #[test]
    fn a_boundary_unregistered_mid_flush_is_not_relaid_with_stale_constraints() {
        let mut d = Driver::new();

        // One tight layout boxes and registers the child, caching tight constraints on its cell.
        d.relay();
        let scope = d.probe_captured.borrow().clone().expect("laid out once");
        assert!(d.probe_constraints.get().expect("laid out once").is_tight());

        // Mark the inner boundary, then flip the holder loose; the root's relayout in the same frame
        // lays the child loosely and unregisters it, so the inner mark is moot.
        scope.mark_needs_layout();
        d.tight.set(false);
        d.relay();

        let last = d.probe_constraints.get().expect("laid out");
        assert!(
            !last.is_tight(),
            "the loose layout from the parent is final; the stale tight entry must not re-lay the child"
        );
    }

    #[test]
    fn a_fast_oscillating_child_never_recovers() {
        let k = u32::from(RelayoutRenderNode::<Probe, Option<Size>>::UNBOX_AFTER_LOOSE_LAYOUTS);

        let mut d = Driver::new();

        // Flip tight and loose every layout for far more iterations than the threshold.
        for i in 0..(k * 8) {
            d.tight.set(i % 2 == 0);
            d.relay();

            assert!(
                !d.inline_after.get(),
                "a child flipping tight and loose every layout never reaches the unbox threshold"
            );

            if i % 2 == 0 {
                assert!(
                    d.registered_after.get(),
                    "a tight layout registers the boundary"
                );
            } else {
                assert!(
                    !d.registered_after.get(),
                    "a loose layout drops the registration but stays boxed"
                );
            }
        }
    }

    #[test]
    fn a_child_that_stays_tight_stays_a_registered_boundary() {
        let mut d = Driver::new();

        for _ in 0..16 {
            d.relay();
            assert!(
                !d.inline_after.get() && d.registered_after.get(),
                "a child constrained tightly every layout stays a registered boundary"
            );
        }
    }
}
