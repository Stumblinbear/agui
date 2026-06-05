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
}

impl AnimatedTransform<()> {
    /// Builds an animation that transforms its subtree by `transform` sampled at each frame's time.
    pub fn new(transform: impl Fn(Duration) -> Affine + 'static) -> Self {
        Self {
            child: (),
            transform: Rc::new(transform),
        }
    }

    pub fn child<Child>(self, child: Child) -> AnimatedTransform<Child> {
        AnimatedTransform {
            child,
            transform: self.transform,
        }
    }
}

impl<Child> Widget for AnimatedTransform<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderAnimatedTransform<Child::Render>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        SingleChildElement::new(&self.child, ctx)
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.update(&self.child, &old.child, ctx);
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(&self.child, path, action)
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        RenderAnimatedTransform::new(
            element.create_render_object(&self.child),
            Rc::clone(&self.transform),
        )
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        element.update_render_object(&self.child, &mut render_object.child);
    }
}

/// The render object of an [`AnimatedTransform`]: applies a per-frame transform to its subtree without
/// repainting it.
pub struct RenderAnimatedTransform<Child> {
    child: Child,
    transform: TransformFn,
    layer: Option<LayerHandle<TransformLayer>>,
    handle: Option<VsyncHandle>,
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

        self.handle =
            Some(vsync.on_frame(move |now| layer.borrow_mut().set_transform(transform(now))));
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
        self.child.hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let layer = self.build_layer();
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
        test_harness::TestHarness,
    };

    use typed_floats::{Positive, PositiveFinite, as_const};

    use super::AnimatedTransform;

    /// A leaf that records how many times it painted, so a test can prove the cached subtree is not
    /// repainted as the transform animates.
    struct Counter {
        paints: Rc<Cell<usize>>,
    }

    struct CounterElement;

    impl agui_core::element::Element for CounterElement {}

    impl Widget for Counter {
        type Element = CounterElement;
        type Render = RenderCounter;

        fn create_element(&self, _: &mut UpdateCtx) -> CounterElement {
            CounterElement
        }

        fn update(&self, _: &mut CounterElement, _: &Self, _: &mut UpdateCtx) {}

        fn dispatch(&self, _: &mut CounterElement, _: &[RoutingId], _: Dispatch) {}

        fn create_render_object(&self, _: &CounterElement) -> RenderCounter {
            RenderCounter {
                paints: Rc::clone(&self.paints),
            }
        }

        fn update_render_object(&self, _: &CounterElement, _: &mut RenderCounter) {}
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

        let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
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
}
