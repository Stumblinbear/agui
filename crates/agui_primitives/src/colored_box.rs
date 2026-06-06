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
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderColoredBox<Child::Render>;

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
        RenderColoredBox {
            color: self.color,

            child: RenderNode::new(element.create_render_object(&self.child)),
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        // TODO(trevin): mark it for repaint if the color has changed
        render_object.color = self.color;

        element.update_render_object(&self.child, &mut render_object.child.object);
    }
}

pub struct RenderColoredBox<Child> {
    color: Color,

    child: RenderNode<Child, Option<Size>>,
}

impl<Child> RenderObject for RenderColoredBox<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
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

        let mut canvas = ctx.canvas();
        let brush = canvas.brush(self.color);

        canvas.fill(Fill::NonZero, brush, &(offset & size));
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
        test_harness::TestHarness,
    };

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn paints_its_color_over_the_child_bounds() {
        let widget =
            ColoredBox::new(Color::rgb8(255, 0, 0)).child(SizedBox::new().width(20).height(10));
        let mut render_object =
            widget.create_render_object(&TestHarness::mount(&widget).root.element);

        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 100, 0, 100),
        );

        let root = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&root, |ctx| render_object.paint(ctx, Offset::ZERO));
        let scene = Compositor::compose(&root).flatten();

        assert_eq!(scene.len(), 1, "fills once, child paints nothing");

        let PaintCommand::Fill { brush, shape, .. } = &scene.commands()[0] else {
            panic!("expected a fill");
        };

        assert!(matches!(scene.brush(*brush), Brush::Solid(c) if *c == Color::rgb8(255, 0, 0)));

        let PaintShape::Rect(rect) = shape else {
            panic!("expected the bounds to be kept as a primitive rect, not flattened to a path");
        };

        assert_eq!(*rect, kurbo::Rect::new(0.0, 0.0, 20.0, 10.0));
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
            .run(&ColoredBox::new(Color::BLACK).child(SizedBox::new().width(20).height(10)));
    }
}
