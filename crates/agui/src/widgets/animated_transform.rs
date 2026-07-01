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
        self.child.child_data = Some(size);
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
            .child_data
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
            .child_data
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

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        let size = self.child.child_data.unwrap_or(Size::ZERO);

        s.with_transform(self.effective(size), |s| {
            self.child.build_semantics(s);
        });
    }
}
