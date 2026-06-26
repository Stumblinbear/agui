use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use typed_floats::{Positive, PositiveFinite};

use crate::{
    paint::{
        compositing::{LayerHandle, SurfaceTransformLayer},
        peniko::kurbo::Affine,
    },
    prelude::{element::*, render_object::*},
    scheduling::{Vsync, VsyncHandle},
};

/// A function from the current frame's time to the transform the subtree should have on it.
pub type TransformFn = Rc<dyn Fn(Duration) -> Affine>;

/// A widget that applies a per-frame transform to its subtree.
///
/// The transform is resampled from the frame's time each frame and applied around a pivot, which is `origin`
/// plus the point `alignment` names within the child. The subtree is composited into a retained layer and
/// moved by recompositing, so it is not repainted as the transform animates. Drive the animation by giving it
/// a [`vsync`](Self::vsync); without one it holds the transform at time zero.
pub struct AnimatedTransform<Child> {
    transform: TransformFn,
    origin: Offset,
    alignment: Alignment,

    vsync: Option<Vsync>,

    child: Child,
}

impl AnimatedTransform<()> {
    /// Builds an animation that transforms its subtree by `transform` sampled at each frame's time.
    pub fn new(transform: impl Fn(Duration) -> Affine + 'static) -> Self {
        Self {
            transform: Rc::new(transform),
            origin: Offset::ZERO,
            alignment: Alignment::TOP_LEFT,
            vsync: None,

            child: (),
        }
    }

    /// Drives the animation from `vsync`.
    pub fn vsync(mut self, vsync: Vsync) -> Self {
        self.vsync = Some(vsync);
        self
    }

    /// Sets the pivot the transform is applied around, as an offset from the child's top-left.
    pub fn origin(mut self, origin: Offset) -> Self {
        self.origin = origin;
        self
    }

    /// Sets the point within the child the transform pivots around, in addition to `origin`.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn child<Child>(self, child: Child) -> AnimatedTransform<Child> {
        AnimatedTransform {
            child,
            transform: self.transform,
            origin: self.origin,
            alignment: self.alignment,
            vsync: self.vsync,
        }
    }
}

impl<Child> Widget for AnimatedTransform<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderAnimatedTransform<Child::Render>>;

    type Render = RenderAnimatedTransform<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let semantics_scope = ctx.deferred_semantics_scope();

        SingleChildElement::new(
            ctx,
            self.child,
            RenderAnimatedTransform {
                transform: self.transform,
                origin: self.origin,
                alignment: self.alignment,

                now: Rc::new(Cell::new(Duration::ZERO)),
                paint_scope: PaintScope::detached(),
                deferred_paint_scope: DeferredPaintScope::detached(),
                semantics: semantics_scope,
                vsync: self.vsync,
                animation: None,

                layer: None,

                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        if !Rc::ptr_eq(&render.transform, &self.transform)
            || render.origin != self.origin
            || render.alignment != self.alignment
        {
            render.transform = self.transform;
            render.origin = self.origin;
            render.alignment = self.alignment;

            render.vsync = self.vsync;

            // The inputs changed, so the subscription must be rebuilt to capture them.
            render.animation = None;
        }

        element.update(ctx, self.child);

        // The subtree is composited into a retained layer, so a rebuild of it while idle needs the layer
        // refilled. An animating transform already marks this every frame; this covers the idle case.
        ctx.mark_needs_paint(element.render_object_mut().paint_scope);
    }
}

/// The render object of an [`AnimatedTransform`]: resamples its transform each frame and recomposites its
/// child under it through a retained layer.
pub struct RenderAnimatedTransform<Child: ?Sized> {
    transform: TransformFn,
    origin: Offset,
    alignment: Alignment,

    /// The frame time the transform is sampled at, advanced by the animation each frame.
    now: Rc<Cell<Duration>>,

    paint_scope: PaintScope,

    /// The deferred handle the animation marks each frame, captured during paint alongside the scope.
    deferred_paint_scope: DeferredPaintScope,

    /// The semantics boundary captured at create, marked by the animation when a tick moves the layer.
    semantics: DeferredSemanticsScope,

    vsync: Option<Vsync>,
    animation: Option<VsyncHandle>,

    /// The layer the child paints into, retained so the animation moves it without a repaint.
    layer: Option<LayerHandle<SurfaceTransformLayer>>,

    child: RenderNode<Child, Option<Size>>,
}

/// The transform with its pivot folded in, in the child's coordinate space.
fn fold_pivot(raw: Affine, origin: Offset, alignment: Alignment, size: Size) -> Affine {
    let pivot = origin + alignment.along_size(size);

    if pivot == Offset::ZERO {
        return raw;
    }

    Affine::translate(pivot) * raw * Affine::translate(-pivot)
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderAnimatedTransform<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: ?Sized> RenderAnimatedTransform<Child> {
    /// The transform to paint and hit-test under, sampled at the current frame and pivoted to `size`.
    fn effective(&self, size: Size) -> Affine {
        fold_pivot(
            (self.transform)(self.now.get()),
            self.origin,
            self.alignment,
            size,
        )
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderAnimatedTransform<Child> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        let size = self.child.parent_data.unwrap_or(Size::ZERO);

        s.with_transform(self.effective(size), |s| {
            self.child.build_semantics(s);
        });
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("origin", self.origin)
            .property("alignment", self.alignment)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderAnimatedTransform<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.child.measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        let size = self.child.layout_and_get_size(ctx, constraints);
        self.child.parent_data = Some(size);
        size
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        let size = self
            .child
            .parent_data
            .expect("animated transform has not been laid out");

        result.with_transform(self.effective(size), position, |result, local| {
            self.child.hit_test(result, local)
        })
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits();

        // An animated transform always composites its child as a group, so it can move the group by
        // recompositing instead of repainting.
        true
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.paint_scope = ctx.scope();
        self.deferred_paint_scope = ctx.deferred_paint_scope();

        let size = self
            .child
            .parent_data
            .expect("animated transform has not been laid out");
        let transform = self.effective(size);

        // Reuse the retained layer across repaints, creating it on the first paint. Clearing drops the
        // previous repaint's child drawing so the child paints in fresh, and is a noop on a new layer.
        let layer = self
            .layer
            .get_or_insert_with(|| LayerHandle::new(SurfaceTransformLayer::new(transform)))
            .clone();
        {
            let mut guard = layer.borrow_mut();
            guard.clear();
            guard.set_transform(transform);
        }

        // Begin animating on the first paint, once the subtree has been laid out. The subscription moves the
        // retained layer and recomposites, so the subtree is not repainted as it animates.
        if self.animation.is_none()
            && let Some(vsync) = self.vsync.as_ref()
        {
            let now = Rc::clone(&self.now);
            let transform = Rc::clone(&self.transform);
            let origin = self.origin;
            let alignment = self.alignment;
            let layer = layer.clone();
            let deferred = self.deferred_paint_scope.clone();
            let semantics = self.semantics.clone();

            self.animation = Some(vsync.on_frame(move |frame| {
                now.set(frame);

                if !layer.borrow_mut().set_transform(fold_pivot(
                    (transform)(frame),
                    origin,
                    alignment,
                    size,
                )) {
                    // The layer moved by recompositing without a repaint, so the subtree's semantics
                    // geometry moved with it. Ask the view to re-read it.
                    deferred.mark_needs_composite();
                    semantics.mark_needs_semantics_update();
                }
            }));
        }

        ctx.push_layer(layer, offset, |ctx| self.child.paint(ctx, Offset::ZERO));
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use peniko::{Color, Fill, kurbo::Affine};
    use typed_floats::{Positive, PositiveFinite, as_const};

    use crate::{
        paint::{command::PaintCommand, scene::Scene},
        pipeline::PipelineOwner,
        prelude::{element::*, render_object::*},
        scheduling::Vsync,
        semantics::{Role, Semantics},
        test_harness::TestCtx,
        view::ViewHandle,
        widgets::{listener::Listener, repaint_boundary::RepaintBoundary, sized_box::SizedBox},
    };

    use super::AnimatedTransform;

    /// A leaf that counts its paints and draws a fill, so a test can see how often the subtree under a
    /// transform is repainted.
    struct Counter {
        paints: Rc<Cell<usize>>,
    }

    impl Widget for Counter {
        type Element = LeafElement<RenderCounter>;

        type Render = RenderCounter;

        fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
            LeafElement::new(RenderCounter {
                paints: self.paints,
            })
        }

        fn update(self, _ctx: &mut UpdateCtx, _element: &mut Self::Element) {}
    }

    struct RenderCounter {
        paints: Rc<Cell<usize>>,
    }

    impl RenderObject for RenderCounter {
        fn build_semantics(&mut self, _s: &mut SemanticsTreeBuilder<'_>) {}

        fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
            d.node_for::<Self>().finish()
        }
    }

    impl RenderBox for RenderCounter {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }

        fn measure(&self, _: BoxConstraints) -> Size {
            Size::new(10.0, 10.0)
        }

        fn layout(&mut self, _: &mut LayoutCtx, _: BoxConstraints) -> Size {
            Size::new(10.0, 10.0)
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

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.paints.set(self.paints.get() + 1);
            let mut canvas = ctx.canvas();
            let brush = canvas.brush(Color::BLACK);
            canvas.fill(Fill::NonZero, brush, &(offset & Size::new(10.0, 10.0)));
        }
    }

    /// The transform in effect at the single fill of a composed scene.
    fn only_fill_transform(scene: &Scene) -> Affine {
        let flat = scene.flatten();

        let mut current = Affine::IDENTITY;
        let mut stack = Vec::new();
        for command in flat.commands() {
            match command {
                PaintCommand::PushTransform(transform) => {
                    stack.push(current);
                    current *= *transform;
                }
                PaintCommand::PopTransform => current = stack.pop().expect("balanced"),
                PaintCommand::Fill { .. } => return current,
                _ => {}
            }
        }

        panic!("expected a fill, got {:?}", flat.commands());
    }

    fn mount(
        widget: impl Widget<Render: RenderBox + Sized + 'static> + 'static,
    ) -> (PipelineOwner, ViewHandle) {
        let (owner, view) = TestCtx::new().mount_view(widget);

        view.resize(BoxConstraints::new(0, 100, 0, 100));

        (owner, view)
    }

    /// Each tick resamples the transform and recomposites the retained layer at the new transform; the
    /// subtree under it is painted once and replayed, not repainted.
    #[test]
    fn the_transform_recomposites_without_repainting_the_subtree() {
        let vsync = Vsync::new();
        let paints = Rc::new(Cell::new(0usize));

        let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
            .vsync(vsync.clone())
            .child(Counter {
                paints: Rc::clone(&paints),
            });

        let (mut owner, view) = mount(widget);
        owner.flush_layout();
        owner.flush_paint();
        assert_eq!(paints.get(), 1);

        vsync.tick(Duration::from_millis(16));
        owner.flush_paint();
        assert_eq!(
            only_fill_transform(&view.composite_frame().rasterize()),
            Affine::translate((16.0, 0.0))
        );

        vsync.tick(Duration::from_millis(32));
        owner.flush_paint();
        assert_eq!(
            only_fill_transform(&view.composite_frame().rasterize()),
            Affine::translate((32.0, 0.0))
        );

        assert_eq!(
            paints.get(),
            1,
            "the subtree was painted once and replayed at each transform"
        );
    }

    /// Moving the layer by recompositing, with no repaint, still asks the view to re-read semantics because
    /// the subtree's geometry moved with it.
    #[test]
    fn moving_the_layer_marks_the_view_semantics() {
        let vsync = Vsync::new();

        let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
            .vsync(vsync.clone())
            .child(Counter {
                paints: Rc::new(Cell::new(0)),
            });

        let (mut owner, _view) = mount(widget);
        owner.flush_layout();
        owner.flush_paint();

        let fired = Rc::new(Cell::new(false));
        let flag = Rc::clone(&fired);
        owner.on_needs_semantics_update(Box::new(move || flag.set(true)));

        vsync.tick(Duration::from_millis(16));
        owner.flush_layout();

        assert!(
            fired.get(),
            "recompositing the moved layer asked the view to re-read semantics"
        );
    }

    /// A flush re-walks each boundary marked since the last frame, hands its rebuilt tree to the closure,
    /// and clears the dirty set so the next change fires again.
    #[test]
    fn flushing_semantics_rewalks_the_marked_boundary() {
        let vsync = Vsync::new();

        let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
            .vsync(vsync.clone())
            .child(
                Semantics::new()
                    .role(Role::Button)
                    .label("Submit")
                    .child(Counter {
                        paints: Rc::new(Cell::new(0)),
                    }),
            );

        let (mut owner, _view) = mount(widget);
        owner.flush_layout();
        owner.flush_paint();

        // Moving the layer marks the view's semantics boundary. `flush_layout` drains that mark into the
        // dirty set the next flush reads.
        vsync.tick(Duration::from_millis(16));
        owner.flush_layout();

        let mut trees = Vec::new();
        owner.flush_semantics(|_id, tree| trees.push(tree));

        assert_eq!(trees.len(), 1, "the one marked boundary was re-walked");
        assert!(
            trees[0].find_by_name("Submit").is_some(),
            "the re-walk captured the subtree's semantics"
        );

        let mut again = 0;
        owner.flush_semantics(|_, _| again += 1);
        assert_eq!(again, 0, "the flush cleared the dirty set");
    }

    /// Wrapping the subtree in a repaint boundary reuses its painting: the transform animates while the
    /// subtree paints once.
    #[test]
    fn a_repaint_boundary_child_paints_once_across_the_animation() {
        let vsync = Vsync::new();
        let paints = Rc::new(Cell::new(0usize));

        let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
            .vsync(vsync.clone())
            .child(RepaintBoundary::new(Counter {
                paints: Rc::clone(&paints),
            }));

        let (mut owner, view) = mount(widget);
        owner.flush_layout();
        owner.flush_paint();
        assert_eq!(paints.get(), 1);

        vsync.tick(Duration::from_millis(16));
        owner.flush_paint();
        assert_eq!(
            only_fill_transform(&view.composite_frame().rasterize()),
            Affine::translate((16.0, 0.0))
        );

        vsync.tick(Duration::from_millis(32));
        owner.flush_paint();
        assert_eq!(
            paints.get(),
            1,
            "the boundary reused its painting across the animation"
        );
    }

    /// A hit is localized through the current transform, so a quarter-turn routes a point that lies only
    /// within the rotated bounds to the child.
    #[test]
    fn a_hit_is_localized_through_the_current_transform() {
        use std::f64::consts::FRAC_PI_2;

        let widget = AnimatedTransform::new(|_| Affine::rotate(FRAC_PI_2)).child(
            Listener::builder()
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(50).height(50)),
        );

        let (mut owner, view) = mount(widget);
        owner.flush_layout();

        // A quarter-turn about the origin places the 50x50 child at x in [-50, 0]. The point (-5, 5) lies
        // outside the unrotated bounds but inside the rotated ones, localizing to the child.
        let result = view.hit_test(Offset::new(-5.0, 5.0));

        assert!(!result.path().is_empty(), "the rotated child is hit");
    }

    /// Aligning the pivot to the child's center rotates it in place: a half-turn about the center keeps the
    /// 50x50 child within its own bounds, so a far-corner hit still lands.
    #[test]
    fn the_pivot_aligns_to_the_child() {
        use std::f64::consts::PI;

        let widget = AnimatedTransform::new(|_| Affine::rotate(PI))
            .alignment(Alignment::CENTER)
            .child(
                Listener::builder()
                    .behavior(HitTestBehavior::Opaque)
                    .child(SizedBox::new().width(50).height(50)),
            );

        let (mut owner, view) = mount(widget);
        owner.flush_layout();

        // A half-turn about the center maps the child's (10, 10) to (40, 40); without the center pivot it
        // would map about the origin and leave the bounds entirely.
        let result = view.hit_test(Offset::new(40.0, 40.0));

        assert!(!result.path().is_empty(), "the rotated child is hit");
    }
}
