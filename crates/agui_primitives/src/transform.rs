use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    alignment::Alignment,
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::SingleChildElement,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::{PaintCtx, peniko::kurbo::Affine},
    render_object::{
        LayoutCtx, MountCtx, PaintScope, RenderNode, RenderObject, box_layout::RenderBox,
    },
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    widget::Widget,
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
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderTransform<Child::Render>;

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
        RenderTransform {
            transform: self.transform,
            origin: self.origin,
            alignment: self.alignment,
            scope: None,
            child: RenderNode::new(element.create_render_object(&self.child)),
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        if self.transform != render_object.transform
            || self.origin != render_object.origin
            || self.alignment != render_object.alignment
        {
            render_object.transform = self.transform;
            render_object.origin = self.origin;
            render_object.alignment = self.alignment;

            // A transform is paint-only and never alters compositing, so a plain repaint suffices.
            if let Some(scope) = &render_object.scope {
                scope.mark_needs_paint();
            }
        }

        element.update_render_object(&self.child, &mut render_object.child.object);
    }
}

pub struct RenderTransform<Child> {
    transform: Affine,
    origin: Offset,
    alignment: Alignment,
    scope: Option<PaintScope>,
    child: RenderNode<Child, Option<Size>>,
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
        self.scope = Some(ctx.paint_scope().clone());
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
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

    fn measure(&self, constraints: Constraints) -> Size {
        self.child.measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: Constraints) -> Size {
        let size = self.child.layout_and_get_size(ctx, constraints);
        self.child.parent_data = Some(size);
        size
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
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
        constraints::Constraints,
        hit_test::HitTestBehavior,
        paint::{
            Compositor, ContainerLayer, LayerHandle, PaintCommand, PaintCtx, PaintShape,
            peniko::{Color, kurbo::Affine, kurbo::Point},
        },
        test_harness::TestHarness,
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
        let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
        render.layout(&mut LayoutCtx::detached(), Constraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).flatten();

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
        let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
        render.layout(&mut LayoutCtx::detached(), Constraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).flatten();

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
        let mut flat = flat.create_render_object(&TestHarness::mount(&flat).root.element);
        assert!(
            !flat.update_compositing_bits(),
            "a transform over a flat child does not composite"
        );

        let layered = Transform::rotate(0.5)
            .child(Opacity::new(0.5).child(SizedBox::new().width(10).height(10)));
        let mut layered = layered.create_render_object(&TestHarness::mount(&layered).root.element);
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
        let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
        render.layout(&mut LayoutCtx::detached(), Constraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).flatten();

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
        let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
        render.layout(&mut LayoutCtx::detached(), Constraints::new(0, 100, 0, 100));

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
        let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
        render.layout(&mut LayoutCtx::detached(), Constraints::new(0, 100, 0, 100));

        let mut result = HitTestResult::new();
        let hit = render.hit_test(&mut result, Offset::new(10.0, 10.0));

        assert_eq!(hit, HitTest::Pass);
        assert!(result.path().is_empty());
    }

    /// A non-invertible transform paints nothing.
    #[test]
    fn a_degenerate_transform_paints_nothing() {
        let widget = Transform::scale(0.0).child(boxed());
        let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
        render.layout(&mut LayoutCtx::detached(), Constraints::new(0, 100, 0, 100));
        render.update_compositing_bits();

        let root = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).flatten();

        assert!(
            scene.is_empty(),
            "a zero-determinant transform paints nothing"
        );
    }
}
