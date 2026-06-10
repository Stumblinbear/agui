use std::{cell::Cell, rc::Rc, time::Duration};

use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    paint::peniko::kurbo::Affine,
    prelude::{element::*, render_object::*},
    scheduling::{Vsync, VsyncHandle},
};

/// A function from the current frame's time to the transform the subtree should have on it.
pub type TransformFn = Rc<dyn Fn(Duration) -> Affine>;

/// A widget that applies a per-frame transform to its subtree.
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

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        let mut render = RenderAnimatedTransform::new(child_render, self.transform);
        render.origin = self.origin;
        render.alignment = self.alignment;
        render.vsync = self.vsync;

        (element, render)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        render_object.transform = self.transform;
        render_object.origin = self.origin;
        render_object.alignment = self.alignment;

        render_object.vsync = self.vsync;
        render_object.animation = None;

        element.update(self.child, &mut render_object.child.object, ctx);

        // The transform or subtree may have changed, so the subtree repaints. An animating transform
        // already marks this every frame; this covers a rebuild while idle.
        if let Some(scope) = &render_object.scope {
            scope.mark_needs_paint();
        }
    }
}

/// The render object of an [`AnimatedTransform`]: resamples its transform each frame and repaints its
/// child under it.
pub struct RenderAnimatedTransform<Child> {
    transform: TransformFn,
    origin: Offset,
    alignment: Alignment,

    /// The frame time the transform is sampled at, advanced by the animation each frame.
    now: Rc<Cell<Duration>>,

    scope: Option<PaintScope>,
    vsync: Option<Vsync>,
    animation: Option<VsyncHandle>,

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

impl<Child> SingleChildRenderObject for RenderAnimatedTransform<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        f(&self.child.object)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child: RenderBox> RenderAnimatedTransform<Child> {
    pub fn new(child: Child, transform: TransformFn) -> Self {
        Self {
            transform,
            origin: Offset::ZERO,
            alignment: Alignment::TOP_LEFT,

            now: Rc::new(Cell::new(Duration::ZERO)),
            scope: None,
            vsync: None,
            animation: None,

            child: RenderNode::new(child),
        }
    }

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

impl<Child: RenderBox> RenderObject for RenderAnimatedTransform<Child> {
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.scope = Some(ctx.paint_scope().clone());
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.animation = None;
        self.child.unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("origin", self.origin)
            .property("alignment", self.alignment)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox> RenderBox for RenderAnimatedTransform<Child> {
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

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        // Begin animating on the first paint, once the subtree has been laid out.
        if self.animation.is_none()
            && let Some(vsync) = self.vsync.as_ref()
        {
            let now = Rc::clone(&self.now);
            let scope = self.scope.clone();

            self.animation = Some(vsync.on_frame(move |frame| {
                now.set(frame);

                if let Some(scope) = &scope {
                    scope.mark_needs_paint();
                }
            }));
        }

        let size = self
            .child
            .parent_data
            .expect("animated transform has not been laid out");

        ctx.with_transform(
            self.child.needs_compositing(),
            Affine::translate(offset) * self.effective(size),
            |ctx| self.child.paint(ctx, Offset::ZERO),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        time::Duration,
    };

    use agui_core::{
        paint::{
            command::PaintCommand,
            compositing::{ContainerLayer, LayerHandle},
            peniko::{Color, Fill, kurbo::Affine},
            scene::Scene,
        },
        pipeline::PipelineOwner,
        prelude::{element::*, render_object::*},
        scheduling::Vsync,
        test_harness::with_ctx,
    };

    use typed_floats::{Positive, PositiveFinite, as_const};

    use super::AnimatedTransform;
    use crate::repaint_boundary::RepaintBoundary;

    /// A leaf that counts its paints and draws a fill, so a test can see how often the subtree under a
    /// transform is repainted.
    struct Counter {
        paints: Rc<Cell<usize>>,
    }

    struct CounterElement;

    impl Element for CounterElement {
        type Render = RenderCounter;
    }

    impl Widget for Counter {
        type Element = CounterElement;
        type Render = RenderCounter;

        fn create(self, _: &mut UpdateCtx) -> (CounterElement, RenderCounter) {
            (
                CounterElement,
                RenderCounter {
                    paints: self.paints,
                },
            )
        }

        fn update(self, _: &mut CounterElement, _: &mut RenderCounter, _: &mut UpdateCtx) {}
    }

    struct RenderCounter {
        paints: Rc<Cell<usize>>,
    }

    impl RenderObject for RenderCounter {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
        fn update_compositing_bits(&mut self) -> bool {
            false
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

    fn mount(widget: impl Widget<Render: RenderBox + 'static> + 'static) -> PipelineOwner {
        let (_, render) = with_ctx(|ctx| widget.create(ctx));

        let owner = PipelineOwner::new(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(ContainerLayer::new()),
        );

        owner.resize(BoxConstraints::new(0, 100, 0, 100));

        owner
    }

    /// Each tick resamples the transform and repaints the subtree under it at the new transform.
    #[test]
    fn the_transform_resamples_each_frame() {
        let vsync = Vsync::new();
        let paints = Rc::new(Cell::new(0usize));

        let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
            .vsync(vsync.clone())
            .child(Counter {
                paints: Rc::clone(&paints),
            });

        let mut owner = mount(widget);
        owner.flush_layout();
        owner.flush_paint();

        vsync.tick(Duration::from_millis(16));
        owner.flush_paint();
        assert_eq!(
            only_fill_transform(&owner.composite()),
            Affine::translate((16.0, 0.0))
        );

        vsync.tick(Duration::from_millis(32));
        owner.flush_paint();
        assert_eq!(
            only_fill_transform(&owner.composite()),
            Affine::translate((32.0, 0.0))
        );

        assert!(
            paints.get() >= 3,
            "the subtree repaints as the transform moves"
        );
    }

    /// Wrapping the subtree in a repaint boundary reuses its painting: the transform animates while the
    /// subtree paints once.
    #[test]
    fn a_repaint_boundary_child_paints_once_across_the_animation() {
        let vsync = Vsync::new();
        let paints = Rc::new(Cell::new(0usize));

        let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
            .vsync(vsync.clone())
            .child(RepaintBoundary::new().child(Counter {
                paints: Rc::clone(&paints),
            }));

        let mut owner = mount(widget);
        owner.flush_layout();
        owner.flush_paint();
        assert_eq!(paints.get(), 1);

        vsync.tick(Duration::from_millis(16));
        owner.flush_paint();
        assert_eq!(
            only_fill_transform(&owner.composite()),
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

    /// A hit is localized through the current transform, so a quarter-turn routes a point that lies
    /// only within the rotated bounds to the child.
    #[test]
    fn a_hit_is_localized_through_the_current_transform() {
        use std::f64::consts::FRAC_PI_2;

        use crate::{listener::Listener, sized_box::SizedBox};

        let widget = AnimatedTransform::new(|_| Affine::rotate(FRAC_PI_2)).child(
            Listener::builder()
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(50).height(50)),
        );

        let mut owner = mount(widget);
        owner.flush_layout();

        // A quarter-turn about the origin places the 50x50 child at x in [-50, 0]. The point (-5, 5)
        // lies outside the unrotated bounds but inside the rotated ones, localizing to the child.
        let result = owner.hit_test(Offset::new(-5.0, 5.0));

        assert!(!result.path().is_empty(), "the rotated child is hit");
    }

    /// Aligning the pivot to the child's center rotates it in place: a half-turn about the center keeps
    /// the 50x50 child within its own bounds, so a far-corner hit still lands.
    #[test]
    fn the_pivot_aligns_to_the_child() {
        use std::f64::consts::PI;

        use crate::{listener::Listener, sized_box::SizedBox};

        let widget = AnimatedTransform::new(|_| Affine::rotate(PI))
            .alignment(Alignment::CENTER)
            .child(
                Listener::builder()
                    .behavior(HitTestBehavior::Opaque)
                    .child(SizedBox::new().width(50).height(50)),
            );

        let mut owner = mount(widget);
        owner.flush_layout();

        // A half-turn about the center maps the child's (10, 10) to (40, 40); without the center pivot
        // it would map about the origin and leave the bounds entirely.
        let result = owner.hit_test(Offset::new(40.0, 40.0));

        assert!(!result.path().is_empty(), "the rotated child is hit");
    }
}

#[cfg(test)]
mod harness {
    use agui_core::paint::peniko::kurbo::Affine;
    use agui_test::prelude::*;

    use super::AnimatedTransform;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default().run(|| {
            AnimatedTransform::new(|_| Affine::IDENTITY).child(SizedBox::new().width(20).height(10))
        });
    }
}
