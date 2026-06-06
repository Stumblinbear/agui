use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    paint::{
        command::PaintShape,
        compositing::{LayerHandle, OpacityLayer},
        peniko::kurbo,
    },
    prelude::{element::*, render_object::*},
};

/// A widget that composites its subtree at a reduced `opacity` — `0.0` fully transparent, `1.0` fully
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
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderOpacity<Child::Render>;

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
        RenderOpacity {
            opacity: self.opacity,
            scope: None,
            child: RenderNode::new(element.create_render_object(&self.child)),
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        if self.opacity != render_object.opacity {
            let was_layer = render_object.needs_layer();
            render_object.opacity = self.opacity;
            let now_layer = render_object.needs_layer();

            if let Some(scope) = &render_object.scope {
                // Crossing the threshold where a layer is needed changes the compositing bits.
                if was_layer == now_layer {
                    scope.mark_needs_paint();
                } else {
                    scope.mark_needs_compositing_update();
                }
            }
        }

        element.update_render_object(&self.child, &mut render_object.child.object);
    }
}

pub struct RenderOpacity<Child> {
    opacity: f32,
    scope: Option<PaintScope>,
    child: RenderNode<Child, Option<Size>>,
}

impl<Child> RenderOpacity<Child> {
    /// Whether the current opacity needs a compositing layer — only a partial opacity does.
    fn needs_layer(&self) -> bool {
        self.opacity > 0.0 && self.opacity < 1.0
    }
}

impl<Child> RenderObject for RenderOpacity<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        // Capture the enclosing boundary so a later opacity change can mark it.
        self.scope = Some(ctx.paint_scope().clone());
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        let child = self.child.update_compositing_bits();
        self.needs_layer() || child
    }
}

impl<Child> RenderBox for RenderOpacity<Child>
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
        self.child.hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        if self.opacity <= 0.0 {
            return;
        }

        if self.opacity >= 1.0 {
            self.child.paint(ctx, offset);

            return;
        }

        let size = self
            .child
            .parent_data
            .expect("opacity has not been laid out");
        // The clip is in the layer's own coordinates, since `push_layer` positions the layer at `offset`.
        let clip = PaintShape::Rect(kurbo::Rect::new(
            0.0,
            0.0,
            f64::from(f32::from(size.width)),
            f64::from(f32::from(size.height)),
        ));

        ctx.push_layer(
            LayerHandle::new(OpacityLayer::new(self.opacity, clip)),
            offset,
            |ctx| self.child.paint(ctx, Offset::ZERO),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use agui_core::{
        paint::{command::PaintCommand, compositing::ContainerLayer, peniko::Color},
        pipeline::PipelineOwner,
        prelude::{element::*, render_object::*},
        test_harness::TestHarness,
    };

    use crate::{colored_box::ColoredBox, sized_box::SizedBox};

    use super::*;

    /// The compositing bit is property-dependent: a partial opacity needs a layer, the whole/none
    /// fast paths do not.
    #[test]
    fn the_compositing_bit_tracks_the_opacity() {
        for (opacity, needs) in [(0.0, false), (0.5, true), (1.0, false)] {
            let widget = Opacity::new(opacity).child(SizedBox::new().width(10).height(10));
            let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);

            assert_eq!(
                render.update_compositing_bits(),
                needs,
                "opacity {opacity} needs_compositing"
            );
        }
    }

    /// A partial opacity composites its subtree as a group at that alpha, end to end through the owner.
    #[test]
    fn partial_opacity_composites_the_subtree_at_its_alpha() {
        let widget = Opacity::new(0.5)
            .child(ColoredBox::new(Color::BLACK).child(SizedBox::new().width(10).height(10)));
        let render = widget.create_render_object(&TestHarness::mount(&widget).root.element);

        let mut owner = PipelineOwner::new(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(ContainerLayer::new()),
        );
        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();
        owner.flush_paint();

        let scene = owner.composite().flatten();
        let alpha = scene.commands().iter().find_map(|command| match command {
            PaintCommand::PushLayer { alpha, .. } => Some(*alpha),
            _ => None,
        });

        assert!(
            matches!(alpha, Some(a) if (a - 0.5).abs() < 1e-6),
            "the subtree was composited at 0.5 alpha, got {alpha:?}"
        );
    }
}

#[cfg(test)]
mod harness {
    use agui_test::prelude::*;

    use super::Opacity;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(&Opacity::new(0.5).child(SizedBox::new().width(20).height(10)));
    }
}
