use typed_floats::{Positive, PositiveFinite};

use crate::{
    paint::peniko::{Color, Fill},
    prelude::{element::*, render_object::*},
};

/// A widget that fills its bounds with a color, then paints its child over it.
///
/// It takes the size of its child, so a childless `ColoredBox` collapses to nothing; give it a
/// sized child, or a sizing parent, to cover a region.
pub struct ColoredBox<Child> {
    color: Color,

    child: Child,
}

impl ColoredBox<()> {
    pub fn new(color: Color) -> Self {
        Self { color, child: () }
    }

    pub fn child<Child>(self, child: Child) -> ColoredBox<Child> {
        ColoredBox {
            color: self.color,

            child,
        }
    }
}

impl<Child> Widget for ColoredBox<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderColoredBox<Child::Render>>;

    type Render = RenderColoredBox<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderColoredBox {
                color: self.color,

                paint_scope: PaintScope::detached(),
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        if render.color != self.color {
            render.color = self.color;

            ctx.mark_needs_paint(render.paint_scope);
        }

        element.update(ctx, self.child);
    }
}

pub struct RenderColoredBox<Child: ?Sized> {
    color: Color,

    /// The repaint boundary the box paints under, captured each paint so a color change can mark it.
    paint_scope: PaintScope,
    child: RenderNode<Child, Option<Size>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderColoredBox<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderColoredBox<Child> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("color", self.color)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderColoredBox<Child> {
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
        let child_size = self.child.layout_and_get_size(ctx, constraints);

        self.child.child_data = Some(child_size);

        child_size
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
        let child_size = self
            .child
            .child_data
            .as_ref()
            .expect("child has not been laid out");

        if !child_size.contains(position) {
            return HitTest::Pass;
        }

        self.child.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.paint_scope = ctx.scope();

        let size = self.child.child_data.expect("child has not been laid out");

        if size.is_zero() {
            return;
        }

        {
            let mut canvas = ctx.canvas();
            let brush = canvas.brush(self.color);

            canvas.fill(Fill::NonZero, brush, &(offset & size));
        }

        self.child.paint(ctx, offset);
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }
}
