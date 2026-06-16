use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    paint::peniko::kurbo::Affine,
    prelude::{element::*, render_object::*},
};

/// A widget that applies a 2D transform to its child while painting, without affecting layout.
///
/// The child is laid out and sized as though untransformed; only its painting is transformed. The
/// transform is applied around a pivot, which is `origin` plus the point `alignment` names within the
/// child. With both at their defaults the pivot is the child's top-left corner.
#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Transform<Child> {
    #[builder(finish_fn)]
    child: Child,

    transform: Affine,

    #[builder(default = Offset::ZERO)]
    origin: Offset,

    #[builder(default = Alignment::TOP_LEFT)]
    alignment: Alignment,
}

impl<Child> Transform<Child> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(transform: Affine) -> TransformBuilder<Child, transform_builder::SetTransform> {
        Self::builder().transform(transform)
    }

    /// Rotates the child by `radians` around its pivot.
    pub fn rotate(radians: f64) -> TransformBuilder<Child, transform_builder::SetTransform> {
        Self::new(Affine::rotate(radians))
    }

    /// Scales the child uniformly by `factor` around its pivot.
    pub fn scale(factor: f64) -> TransformBuilder<Child, transform_builder::SetTransform> {
        Self::new(Affine::scale(factor))
    }

    /// Translates the child by `offset`.
    pub fn translate(offset: Offset) -> TransformBuilder<Child, transform_builder::SetTransform> {
        Self::new(Affine::translate(offset))
    }
}

impl<Child> Widget for Transform<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderTransform<Child::Render>>;

    type Render = RenderTransform<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderTransform {
                transform: self.transform,
                origin: self.origin,
                alignment: self.alignment,

                paint_scope: PaintScope::detached(),

                child: RenderNode::new(child_render),
            },
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        if render_object.transform != self.transform
            || render_object.origin != self.origin
            || render_object.alignment != self.alignment
        {
            render_object.transform = self.transform;
            render_object.origin = self.origin;
            render_object.alignment = self.alignment;

            // A transform is paint-only and never alters compositing, so a plain repaint suffices.
            ctx.mark_needs_paint(render_object.paint_scope);
        }

        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

pub struct RenderTransform<Child> {
    transform: Affine,
    origin: Offset,
    alignment: Alignment,

    paint_scope: PaintScope,

    child: RenderNode<Child, Option<Size>>,
}

impl<Child> SingleChildRenderObject for RenderTransform<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        f(&self.child.object)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child> RenderTransform<Child> {
    /// The transform with its pivot folded in, in the child's coordinate space.
    fn effective_transform(&self, size: Size) -> Affine {
        let pivot = self.origin + self.alignment.along_size(size);

        if pivot == Offset::ZERO {
            return self.transform;
        }

        Affine::translate(pivot) * self.transform * Affine::translate(-pivot)
    }
}

/// The pure translation a transform reduces to, if it is one.
#[allow(clippy::float_cmp, clippy::cast_possible_truncation)]
fn as_translation(transform: Affine) -> Option<Offset> {
    let [a, b, c, d, e, f] = transform.as_coeffs();

    (a == 1.0 && b == 0.0 && c == 0.0 && d == 1.0).then(|| Offset::new(e as f32, f as f32))
}

/// Whether a transform maps the child to a visible region.
#[allow(clippy::float_cmp)]
fn is_paintable(transform: Affine) -> bool {
    let determinant = transform.determinant();

    determinant != 0.0 && determinant.is_finite()
}

impl<Child> RenderObject for RenderTransform<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        // Capture the enclosing boundary so a later transform change can mark it.
        self.paint_scope = *ctx.paint_scope();

        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.paint_scope = PaintScope::detached();

        self.child.unmount(ctx);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("transform", self.transform.as_coeffs())
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child> RenderBox for RenderTransform<Child>
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
            .expect("transform has not been laid out");
        let effective = self.effective_transform(size);

        result.with_transform(effective, position, |result, local| {
            self.child.hit_test(result, local)
        })
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let size = self
            .child
            .parent_data
            .expect("transform has not been laid out");
        let effective = self.effective_transform(size);

        // Folding a pure translation into the offset skips a transform command, and any layer, entirely.
        if let Some(translation) = as_translation(effective) {
            self.child.paint(ctx, offset + translation);
            return;
        }

        // A non-invertible transform collapses the child out of view, so there is nothing to paint.
        if !is_paintable(effective) {
            return;
        }

        ctx.with_transform(
            self.child.needs_compositing(),
            Affine::translate(offset) * effective,
            |ctx| self.child.paint(ctx, Offset::ZERO),
        );
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        paint::{
            command::{PaintCommand, PaintShape},
            compositing::{Compositor, LayerHandle, OffsetLayer},
            peniko::{
                Color,
                kurbo::{Affine, Point},
            },
        },
        prelude::{element::*, render_object::*},
        test_harness::TestCtx,
    };

    use crate::{
        colored_box::ColoredBox, listener::Listener, opacity::Opacity, sized_box::SizedBox,
    };

    use super::*;

    fn boxed() -> ColoredBox<SizedBox<()>> {
        ColoredBox::new(Color::BLACK).child(SizedBox::new().width(10).height(10))
    }

    /// A rotation over a non-compositing child draws flat, under a single transform command.
    #[test]
    fn a_rotation_over_a_flat_child_paints_under_a_transform() {
        let widget = Transform::rotate(0.5).child(boxed());
        let mut render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(OffsetLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).rasterize();

        let transforms: Vec<Affine> = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::PushTransform(transform) => Some(*transform),
                _ => None,
            })
            .collect();

        assert_eq!(transforms, vec![Affine::rotate(0.5)]);
    }

    /// A pure translation is folded into the paint offset, emitting no transform command.
    #[test]
    fn a_translation_folds_into_the_offset() {
        let widget = Transform::translate(Offset::new(5.0, 7.0)).child(boxed());
        let mut render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(OffsetLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).rasterize();

        assert!(
            !scene
                .commands()
                .iter()
                .any(|command| matches!(command, PaintCommand::PushTransform(_))),
            "a pure translation should not emit a transform command"
        );

        let fill = scene.commands().iter().find_map(|command| match command {
            PaintCommand::Fill {
                shape: PaintShape::Rect(rect),
                ..
            } => Some(*rect),
            _ => None,
        });

        assert!(
            matches!(fill, Some(rect) if rect.x0 == 5.0 && rect.y0 == 7.0),
            "the fill lands at the translated origin, got {fill:?}"
        );
    }

    /// The compositing bit is the child's: a transform contributes no layer of its own.
    #[test]
    fn the_compositing_bit_is_inherited_from_the_child() {
        let flat = Transform::rotate(0.5).child(boxed());
        let (_, mut flat) = TestCtx::new().create(flat);
        assert!(
            !flat.update_compositing_bits(),
            "a transform over a flat child does not composite"
        );

        let layered = Transform::rotate(0.5)
            .child(Opacity::new(0.5).child(SizedBox::new().width(10).height(10)));
        let (_, mut layered) = TestCtx::new().create(layered);
        assert!(
            layered.update_compositing_bits(),
            "a transform inherits its child's compositing need"
        );
    }

    /// When the child composites, the transform realizes a layer that wraps the child's, rather than
    /// drawing flat. The transform appears outside the child's opacity group.
    #[test]
    fn a_transform_over_a_compositing_child_wraps_its_layer() {
        let widget = Transform::rotate(0.5).child(Opacity::new(0.5).child(boxed()));
        let mut render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(OffsetLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).rasterize();

        let push_transform = scene
            .commands()
            .iter()
            .position(|command| matches!(command, PaintCommand::PushTransform(_)));
        let push_layer = scene
            .commands()
            .iter()
            .position(|command| matches!(command, PaintCommand::PushLayer { .. }));

        assert!(
            matches!((push_transform, push_layer), (Some(t), Some(l)) if t < l),
            "the transform should wrap the opacity layer, got {:?}",
            scene.commands()
        );
    }

    /// A hit is localized through the inverse transform: a translation of (10, 0) maps a root-space
    /// (15, 5) onto the child's (5, 5).
    #[test]
    fn a_hit_is_localized_through_the_inverse_transform() {
        let widget = Transform::translate(Offset::new(10.0, 0.0)).child(
            Listener::builder()
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(50).height(50)),
        );
        let render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 100, 0, 100));

        let mut result = HitTestResult::new();
        let hit = render.hit_test(&mut result, Offset::new(15.0, 5.0));

        assert_eq!(hit, HitTest::Absorb);

        let transform = result.path()[0].global_transform();
        let local = Offset::from(transform * Point::from(Offset::new(15.0, 5.0)));
        assert_eq!(local.x.get(), 5.0);
        assert_eq!(local.y.get(), 5.0);
    }

    /// A non-invertible transform can't be hit: the child is never entered and nothing is recorded.
    #[test]
    fn a_degenerate_transform_is_not_hit() {
        let widget = Transform::scale(0.0).child(
            Listener::builder()
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(50).height(50)),
        );
        let render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 100, 0, 100));

        let mut result = HitTestResult::new();
        let hit = render.hit_test(&mut result, Offset::new(10.0, 10.0));

        assert_eq!(hit, HitTest::Pass);
        assert!(result.path().is_empty());
    }

    /// A non-invertible transform paints nothing.
    #[test]
    fn a_degenerate_transform_paints_nothing() {
        let widget = Transform::scale(0.0).child(boxed());
        let mut render = TestCtx::new().laid_out(widget, BoxConstraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(OffsetLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).rasterize();

        assert!(
            scene.is_empty(),
            "a zero-determinant transform paints nothing"
        );
    }
}

#[cfg(test)]
mod harness {
    use agui_core::paint::peniko::kurbo::Affine;
    use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

    use super::Transform;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_element_lifecycle() {
        ElementLifecycleCheck::new()
            .single_child(|child| Transform::new(Affine::IDENTITY).child(child));
    }

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(|| Transform::new(Affine::IDENTITY).child(SizedBox::new().width(20).height(10)));
    }
}
