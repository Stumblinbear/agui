use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::{
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

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderTransform {
                transform: self.transform,
                origin: self.origin,
                alignment: self.alignment,

                paint_scope: PaintScope::detached(),
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        if render.transform != self.transform
            || render.origin != self.origin
            || render.alignment != self.alignment
        {
            render.transform = self.transform;
            render.origin = self.origin;
            render.alignment = self.alignment;

            // A transform is paint-only and never alters compositing, so a plain repaint suffices.
            ctx.mark_needs_paint(render.paint_scope);
        }

        element.update(ctx, self.child);
    }
}

pub struct RenderTransform<Child: ?Sized> {
    transform: Affine,
    origin: Offset,
    alignment: Alignment,

    paint_scope: PaintScope,
    child: RenderNode<Child, Option<Size>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderTransform<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: ?Sized> RenderTransform<Child> {
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
#[allow(
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::many_single_char_names
)]
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

impl<Child: RenderBox + ?Sized> RenderObject for RenderTransform<Child> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("transform", self.transform.as_coeffs())
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderTransform<Child> {
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
        self.paint_scope = ctx.scope();

        let size = self
            .child
            .child_data
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

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        let size = self.child.child_data.unwrap_or(Size::ZERO);

        s.with_transform(self.effective_transform(size), |s| {
            self.child.build_semantics(s);
        });
    }
}
