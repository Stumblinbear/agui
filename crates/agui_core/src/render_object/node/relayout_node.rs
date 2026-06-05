use std::{cell::RefCell, hint::unreachable_unchecked, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::PaintCtx,
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    pipeline::{
        layout::{BoundaryContent, LayoutScope},
        paint::PaintScope,
    },
    render_object::{
        LayoutCtx, MountCtx, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    text::TextBaseline,
};

/// A child holder whose child becomes a relayout boundary while it is constrained tightly.
///
/// A render object holds a child here instead of in a [`RenderNode`](super::RenderNode) when the child
/// can be laid out on its own: while the constraints handed down are tight, the child's size is fixed by
/// them, so a change confined to its subtree re-lays only the child, not the holder or anything above.
/// The child is monomorphized and free of overhead until the first tight layout; only then is it boxed
/// and registered, and it stays inline if it is never constrained tightly.
pub struct RelayoutRenderNode<R, P = ()> {
    pub parent_data: P,

    child: RelayoutChild<R>,

    paint: Option<PaintScope>,
    parent_uses_size: bool,
    needs_compositing: bool,
}

enum RelayoutChild<R> {
    /// The child held by value, while it has never been constrained tightly.
    Inline(R),

    /// The child erased and shared, once it has been a relayout boundary. `boundary` is the scope that
    /// marks it while it is registered, dropped while loose constraints make it unsound to re-lay alone.
    Boxed {
        content: BoundaryContent,
        boundary: Option<LayoutScope>,
    },
}

enum Form {
    BoxAndRegister,
    Register,
    Unregister,
    Keep,
}

impl<R, P: Default> RelayoutRenderNode<R, P> {
    pub fn new(child: R) -> Self {
        Self {
            parent_data: P::default(),

            child: RelayoutChild::Inline(child),

            paint: None,
            parent_uses_size: false,
            needs_compositing: false,
        }
    }
}

impl<R: RenderBox, P> RelayoutRenderNode<R, P> {
    pub fn mount(&mut self, ctx: &mut MountCtx) {
        self.paint = Some(ctx.paint_scope().clone());

        match &mut self.child {
            RelayoutChild::Inline(child) => child.mount(ctx),
            RelayoutChild::Boxed { content, .. } => content.mount(ctx),
        }
    }

    pub fn unmount(&mut self, ctx: &mut MountCtx) {
        match &mut self.child {
            RelayoutChild::Inline(child) => child.unmount(ctx),
            RelayoutChild::Boxed { content, boundary } => {
                content.unmount(ctx);

                if let Some(boundary) = boundary.take() {
                    boundary.unregister();
                }
            }
        }
    }

    pub fn update_compositing_bits(&mut self) -> bool {
        self.needs_compositing = match &mut self.child {
            RelayoutChild::Inline(child) => child.update_compositing_bits(),
            RelayoutChild::Boxed { content, .. } => content.update_compositing_bits(),
        };

        self.needs_compositing
    }

    /// Whether this child's subtree contributes a compositing layer, as of the last recompute.
    pub fn needs_compositing(&self) -> bool {
        self.needs_compositing
    }

    /// Reconciles the child, recovering its concrete type through the boxed form when it is a boundary.
    ///
    /// # Panics
    ///
    /// Panics if the child's render type has changed since it was created, which a correct reconcile
    /// never does.
    pub fn with_object_mut<T>(&mut self, f: impl FnOnce(&mut R) -> T) -> T {
        match &mut self.child {
            RelayoutChild::Inline(child) => f(child),
            RelayoutChild::Boxed { content, .. } => {
                let mut content = content.borrow_mut();
                let child = content.as_any_mut().downcast_mut::<R>().unwrap();

                f(child)
            }
        }
    }

    pub fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.min_intrinsic_width(height),
            RelayoutChild::Boxed { content, .. } => content.min_intrinsic_width(height),
        }
    }

    pub fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.max_intrinsic_width(height),
            RelayoutChild::Boxed { content, .. } => content.max_intrinsic_width(height),
        }
    }

    pub fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.min_intrinsic_height(width),
            RelayoutChild::Boxed { content, .. } => content.min_intrinsic_height(width),
        }
    }

    pub fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.max_intrinsic_height(width),
            RelayoutChild::Boxed { content, .. } => content.max_intrinsic_height(width),
        }
    }

    pub fn measure(&self, constraints: BoxConstraints) -> Size {
        match &self.child {
            RelayoutChild::Inline(child) => child.measure(constraints),
            RelayoutChild::Boxed { content, .. } => content.measure(constraints),
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
                Some(boundary) => {
                    boundary.update_constraints(constraints);

                    ctx.with_layout_scope(boundary.clone(), |ctx| content.layout(ctx, constraints))
                }

                None => content.layout(ctx, constraints),
            },
        }
    }

    /// Moves the child between its inline and boundary forms to match whether it is now constrained
    /// tightly. Boxing is one-way, since the erased child cannot be recovered by value; only the boundary
    /// registration toggles afterward.
    fn reshape(&mut self, tight: bool, scope: &LayoutScope) {
        // A detached scope cannot register a boundary, so the child stays inline; this also keeps an
        // unmounted layout (a measurement or a test) from needing a paint scope it has not captured.
        let registrable = tight && !scope.is_detached();

        let form = match &self.child {
            RelayoutChild::Inline(_) if registrable => Form::BoxAndRegister,

            RelayoutChild::Boxed { boundary: None, .. } if registrable => Form::Register,

            RelayoutChild::Boxed {
                boundary: Some(_), ..
            } if !tight => Form::Unregister,

            _ => Form::Keep,
        };

        match form {
            Form::BoxAndRegister => {
                let paint = self
                    .paint
                    .clone()
                    .expect("child must be mounted before it is laid out");

                take(&mut self.child, |child| {
                    let RelayoutChild::Inline(child) = child else {
                        // SAFETY: The inline form was just observed, so it must be Inline.
                        unsafe {
                            unreachable_unchecked();
                        }
                    };

                    let content: BoundaryContent = Rc::new(RefCell::new(child));
                    let boundary = scope.register(Rc::clone(&content), paint);

                    RelayoutChild::Boxed {
                        content,
                        boundary: Some(boundary),
                    }
                });
            }

            Form::Register => {
                let paint = self
                    .paint
                    .clone()
                    .expect("child must be mounted before it is laid out");

                if let RelayoutChild::Boxed { content, boundary } = &mut self.child {
                    *boundary = Some(scope.register(Rc::clone(content), paint));
                }
            }

            Form::Unregister => {
                if let RelayoutChild::Boxed { boundary, .. } = &mut self.child
                    && let Some(boundary) = boundary.take()
                {
                    boundary.unregister();
                }
            }

            Form::Keep => {}
        }
    }

    pub fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        match &self.child {
            RelayoutChild::Inline(child) => child.measure_baseline(constraints, baseline),
            RelayoutChild::Boxed { content, .. } => content.measure_baseline(constraints, baseline),
        }
    }

    pub fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        match &mut self.child {
            RelayoutChild::Inline(child) => child.distance_to_baseline(baseline),
            RelayoutChild::Boxed { content, .. } => content.distance_to_baseline(baseline),
        }
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        match &self.child {
            RelayoutChild::Inline(child) => child.hit_test(result, position),
            RelayoutChild::Boxed { content, .. } => content.hit_test(result, position),
        }
    }

    pub fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        match &mut self.child {
            RelayoutChild::Inline(child) => child.paint(ctx, offset),
            RelayoutChild::Boxed { content, .. } => content.paint(ctx, offset),
        }
    }
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
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        paint::compositing::{ContainerLayer, LayerHandle},
        pipeline::PipelineOwner,
        render_object::{
            MountCtx, RenderObject,
            box_layout::{BoxConstraints, RenderBox},
        },
        text::TextBaseline,
    };

    use super::*;

    fn layer() -> LayerHandle<ContainerLayer> {
        LayerHandle::new(ContainerLayer::new())
    }

    type Captured = Rc<RefCell<Option<LayoutScope>>>;

    /// A leaf that counts its layouts and paints and captures the scope it was laid out under, so a test
    /// can re-lay it the way a change inside it would.
    struct Probe {
        layouts: Rc<Cell<usize>>,
        paints: Rc<Cell<usize>>,
        captured: Captured,
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
            }),
        };

        let content: BoundaryContent = Rc::new(RefCell::new(tighten));
        let mut owner = PipelineOwner::new(Rc::clone(&content), layer());

        owner.resize(BoxConstraints::new(0, 100, 0, 100));
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
}
