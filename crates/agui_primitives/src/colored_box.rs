use typed_floats::{Positive, PositiveFinite};

use agui_core::{
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

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderColoredBox {
                color: self.color,

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
        if render_object.color != self.color {
            render_object.color = self.color;

            render_object.paint_scope.mark_needs_paint();
        }

        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

pub struct RenderColoredBox<Child> {
    color: Color,

    paint_scope: PaintScope,

    child: RenderNode<Child, Option<Size>>,
}

impl<Child> SingleChildRenderObject for RenderColoredBox<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        f(&self.child.object)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child> RenderObject for RenderColoredBox<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.paint_scope = ctx.paint_scope().clone();

        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("color", self.color)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child> RenderBox for RenderColoredBox<Child>
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
        let child_size = self.child.layout_and_get_size(ctx, constraints);

        self.child.parent_data = Some(child_size);

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
            .parent_data
            .as_ref()
            .expect("child has not been laid out");

        if !child_size.contains(position) {
            return HitTest::Pass;
        }

        self.child.hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let size = self.child.parent_data.expect("child has not been laid out");

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
}

#[cfg(test)]
mod tests {
    use agui_core::{
        paint::{
            command::{PaintCommand, PaintShape},
            compositing::{Compositor, ContainerLayer, LayerHandle},
            peniko::{Brush, kurbo},
        },
        prelude::{element::*, render_object::*},
        test_harness::with_ctx,
    };

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn paints_its_color_over_the_child_bounds() {
        let widget = ColoredBox::new(Color::from_rgb8(255, 0, 0))
            .child(SizedBox::new().width(20).height(10));
        let (_, mut render_object) = with_ctx(|ctx| widget.create(ctx));

        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 100, 0, 100),
        );

        let root = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&root, |ctx| render_object.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).flatten();

        assert_eq!(scene.len(), 1, "fills once; the empty child paints nothing");

        let PaintCommand::Fill { brush, shape, .. } = &scene.commands()[0] else {
            panic!("expected a fill");
        };

        assert!(
            matches!(scene.brush(*brush), Brush::Solid(c) if *c == Color::from_rgb8(255, 0, 0))
        );

        let PaintShape::Rect(rect) = shape else {
            panic!("expected the bounds to be kept as a primitive rect, not flattened to a path");
        };

        assert_eq!(*rect, kurbo::Rect::new(0.0, 0.0, 20.0, 10.0));
    }

    #[test]
    fn paints_the_child_over_its_color() {
        // A painting child nested inside, so the outer fill must be followed by the child's own.
        let widget = ColoredBox::new(Color::from_rgb8(255, 0, 0)).child(
            ColoredBox::new(Color::from_rgb8(0, 0, 255))
                .child(SizedBox::new().width(20).height(10)),
        );
        let (_, mut render_object) = with_ctx(|ctx| widget.create(ctx));

        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 100, 0, 100),
        );

        let root = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&root, |ctx| render_object.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).flatten();

        let colors: Vec<_> = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                PaintCommand::Fill { brush, .. } => match scene.brush(*brush) {
                    Brush::Solid(color) => Some(*color),
                    _ => None,
                },
                _ => None,
            })
            .collect();

        assert_eq!(
            colors,
            vec![Color::from_rgb8(255, 0, 0), Color::from_rgb8(0, 0, 255)],
            "the color fills first, then the child paints over it"
        );
    }
}

#[cfg(test)]
mod harness {
    use agui_test::prelude::*;

    use super::ColoredBox;
    use crate::sized_box::SizedBox;

    #[test]
    fn paints_within_its_bounds() {
        BoxSizingCheck::default()
            .run(|| ColoredBox::new(Color::BLACK).child(SizedBox::new().width(20).height(10)));
    }
}
