use typed_floats::{Positive, PositiveFinite};

use crate::{
    paint::compositing::{LayerHandle, OpacityLayer},
    prelude::{element::*, render_object::*},
};

/// A widget that composites its subtree at a reduced `opacity`, from `0.0` fully transparent to `1.0` fully
/// opaque.
pub struct Opacity<Child> {
    opacity: f32,
    child: Child,
}

impl Opacity<()> {
    pub fn new(opacity: f32) -> Self {
        Self { opacity, child: () }
    }

    pub fn child<Child>(self, child: Child) -> Opacity<Child> {
        Opacity {
            opacity: self.opacity,
            child,
        }
    }
}

impl<Child> Widget for Opacity<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderOpacity<Child::Render>>;

    type Render = RenderOpacity<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderOpacity {
                opacity: self.opacity,

                paint_scope: PaintScope::detached(),
                layer: None,
                child: RenderNode::new(()),
            },
        )
    }

    // Exact comparison is intended: any change in opacity, however small, must update the subtree.
    #[allow(clippy::float_cmp)]
    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        if render.opacity != self.opacity {
            let was_layer = render.needs_layer();
            render.opacity = self.opacity;
            let now_layer = render.needs_layer();

            if was_layer && now_layer {
                // Still partial: poke the retained layer's alpha in place; the next composite picks it up.
                if let Some(layer) = &render.layer {
                    layer.borrow_mut().set_alpha(self.opacity);
                }
            } else if was_layer == now_layer {
                // Both fully transparent or fully opaque: a visibility flip still repaints.
                ctx.mark_needs_paint(render.paint_scope);
            } else {
                // Crossing the threshold where a layer is needed changes the compositing bits.
                ctx.mark_needs_compositing_bits_update(render.paint_scope);
            }
        }

        element.update(ctx, self.child);
    }
}

pub struct RenderOpacity<Child: ?Sized> {
    opacity: f32,

    paint_scope: PaintScope,

    /// The layer painted at the last partial-opacity paint, retained so an opacity change that stays
    /// partial can recomposite it at the new alpha without repainting the subtree.
    layer: Option<LayerHandle<OpacityLayer>>,

    child: RenderNode<Child>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderOpacity<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: ?Sized> RenderOpacity<Child> {
    /// Whether the current opacity needs a compositing layer. Only a partial opacity does.
    fn needs_layer(&self) -> bool {
        self.opacity > 0.0 && self.opacity < 1.0
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderOpacity<Child> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("opacity", self.opacity)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderOpacity<Child> {
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
        self.child.layout_and_get_size(ctx, constraints)
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

    fn update_compositing_bits(&mut self) -> bool {
        let child = self.child.update_compositing_bits();
        self.needs_layer() || child
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.paint_scope = ctx.scope();

        if self.opacity <= 0.0 {
            self.layer = None;
            return;
        }

        if self.opacity >= 1.0 {
            self.layer = None;
            self.child.paint(ctx, offset);

            return;
        }

        // A reduced opacity fades its subtree without clipping it, so a transformed descendant that
        // paints outside the box stays visible.
        //
        // Retain the layer so a later partial-to-partial opacity change can recomposite it in place.
        let layer = LayerHandle::new(OpacityLayer::new(self.opacity));
        self.layer = Some(layer.clone());

        ctx.push_layer(layer, offset, |ctx| self.child.paint(ctx, Offset::ZERO));
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }
}
