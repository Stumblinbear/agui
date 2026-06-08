use std::{rc::Rc, time::Duration};

use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    paint::{
        compositing::{LayerHandle, TransformLayer},
        peniko::kurbo::Affine,
    },
    prelude::{element::*, render_object::*},
    scheduling::{Vsync, VsyncHandle},
};

/// A function from the current frame's time to the transform the subtree should have on it.
pub type TransformFn = Rc<dyn Fn(Duration) -> Affine>;

/// A widget that applies a per-frame transform to its subtree.
///
/// Use it to move, scale, or rotate a subtree continuously; the subtree is not repainted as it
/// animates.
pub struct AnimatedTransform<Child> {
    child: Child,
    transform: TransformFn,
    vsync: Option<Vsync>,
}

impl AnimatedTransform<()> {
    /// Builds an animation that transforms its subtree by `transform` sampled at each frame's time.
    pub fn new(transform: impl Fn(Duration) -> Affine + 'static) -> Self {
        Self {
            child: (),
            transform: Rc::new(transform),
            vsync: None,
        }
    }

    /// Drives the animation from `vsync`: the transform is resampled and reapplied each frame the
    /// registry ticks. Without one, the subtree keeps the transform sampled at the first frame.
    pub fn vsync(mut self, vsync: Vsync) -> Self {
        self.vsync = Some(vsync);
        self
    }

    pub fn child<Child>(self, child: Child) -> AnimatedTransform<Child> {
        AnimatedTransform {
            child,
            transform: self.transform,
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
        let Self {
            child,
            transform,
            vsync,
        } = self;

        let (element, child_render) = SingleChildElement::new(child, ctx);

        let mut render = RenderAnimatedTransform::new(child_render, transform);
        render.vsync = vsync;

        (element, render)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        element.update(self.child, &mut render_object.child, ctx);
    }
}

/// The render object of an [`AnimatedTransform`]: applies a per-frame transform to its subtree without
/// repainting it.
pub struct RenderAnimatedTransform<Child> {
    transform: TransformFn,
    layer: Option<LayerHandle<TransformLayer>>,
    handle: Option<VsyncHandle>,
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

    fn with_child<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child> RenderAnimatedTransform<Child>
where
    Child: RenderBox,
{
    pub fn new(child: Child, transform: TransformFn) -> Self {
        Self {
            child,
            transform,
            layer: None,
            handle: None,
            vsync: None,
        }
    }

    /// Builds and returns this subtree's transform layer, painting the subtree once. Call after layout;
    /// repeated calls return the same layer.
    pub fn build_layer(&mut self) -> LayerHandle<TransformLayer> {
        if let Some(layer) = &self.layer {
            return layer.clone();
        }

        let layer = LayerHandle::new(TransformLayer::new((self.transform)(Duration::ZERO)));
        PaintCtx::paint(&layer, |ctx| self.child.paint(ctx, Offset::ZERO));

        self.layer = Some(layer.clone());

        layer
    }

    /// Starts the animation: the transform is resampled and applied each frame, until this render
    /// object unmounts or is dropped.
    pub fn animate(&mut self, vsync: &Vsync) {
        let layer = self.build_layer();
        let transform = Rc::clone(&self.transform);

        self.handle = Some(vsync.on_frame(move |now| {
            layer.borrow_mut().set_transform(transform(now));
        }));
    }

    /// This subtree's transform layer, once it has been built.
    pub fn layer(&self) -> Option<LayerHandle<TransformLayer>> {
        self.layer.clone()
    }
}

impl<Child> RenderObject for RenderAnimatedTransform<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        // Stop animating the moment the subtree leaves the tree.
        self.handle = None;
        self.child.unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        // The subtree is painted into a retained transform layer, so this node always composites.
        true
    }
}

impl<Child> RenderBox for RenderAnimatedTransform<Child>
where
    Child: RenderBox,
{
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
        self.child.layout(ctx, constraints)
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
        // Localize through the transform the layer currently shows, so a hit lands where the
        // animated subtree is drawn rather than where it was laid out.
        let transform = self
            .layer
            .as_ref()
            .map_or(Affine::IDENTITY, |layer| layer.borrow().transform());

        result.with_transform(transform, position, |result, local| {
            self.child.hit_test(result, local)
        })
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let layer = self.build_layer();

        // Begin animating on the first paint, once the layer exists and the subtree has been laid out.
        if self.handle.is_none()
            && let Some(vsync) = self.vsync.clone()
        {
            self.animate(&vsync);
        }

        ctx.add_layer(layer.into(), offset);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc, time::Duration};

    use agui_core::{
        paint::{
            command::PaintCommand,
            compositing::Compositor,
            peniko::{Color, Fill, kurbo::Affine},
            scene::Scene,
        },
        prelude::{element::*, render_object::*},
        scheduling::Vsync,
        test_harness::with_ctx,
    };

    use typed_floats::{Positive, PositiveFinite, as_const};

    use super::AnimatedTransform;

    /// A leaf that records how many times it painted, so a test can prove the cached subtree is not
    /// repainted as the transform animates.
    struct Counter {
        paints: Rc<Cell<usize>>,
    }

    struct CounterElement;

    impl agui_core::element::Element for CounterElement {
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

    /// Ticking vsync re-places the subtree at the new transform; the subtree is painted exactly once,
    /// no matter how many frames the transform animates over.
    #[test]
    fn transform_animates_via_vsync_without_repainting() {
        let vsync = Vsync::new();
        let paints = Rc::new(Cell::new(0usize));

        let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
            .child(Counter {
                paints: Rc::clone(&paints),
            });

        let (_, mut render) = with_ctx(|ctx| widget.create(ctx));
        render.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 100, 0, 100),
        );

        render.build_layer();
        assert_eq!(
            paints.get(),
            1,
            "building the layer paints the subtree once"
        );

        render.animate(&vsync);

        vsync.tick(Duration::from_millis(16));
        let scene = Compositor::compose(&render.layer().unwrap());
        assert_eq!(only_fill_transform(&scene), Affine::translate((16.0, 0.0)));
        assert_eq!(paints.get(), 1, "no repaint after the first frame");

        vsync.tick(Duration::from_millis(32));
        let scene = Compositor::compose(&render.layer().unwrap());
        assert_eq!(only_fill_transform(&scene), Affine::translate((32.0, 0.0)));
        assert_eq!(
            paints.get(),
            1,
            "the subtree painted once across the whole animation"
        );
    }

    /// A hit is localized through the transform the layer shows, so a quarter-turn routes a point
    /// that lies only within the rotated bounds to the child.
    #[test]
    fn a_hit_is_localized_through_the_current_transform() {
        use std::f64::consts::FRAC_PI_2;

        use crate::{listener::Listener, sized_box::SizedBox};

        let widget = AnimatedTransform::new(|_| Affine::rotate(FRAC_PI_2)).child(
            Listener::builder()
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(50).height(50)),
        );

        let (_, mut render) = with_ctx(|ctx| widget.create(ctx));
        render.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 100, 0, 100),
        );
        // Build the layer so it carries the rotation the hit test reads.
        render.build_layer();

        // A quarter-turn about the origin places the 50x50 child at x in [-50, 0]. The point
        // (-5, 5) lies outside the unrotated bounds but inside the rotated ones, localizing to the
        // child's (5, 5).
        let mut result = HitTestResult::new();
        let hit = render.hit_test(&mut result, Offset::new(-5.0, 5.0));

        assert_eq!(hit, HitTest::Absorb);
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
