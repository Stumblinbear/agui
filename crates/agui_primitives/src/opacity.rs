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
    type Element = SingleChildElement<Child::Element, RenderOpacity<Child::Render>>;

    type Render = RenderOpacity<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderOpacity {
                opacity: self.opacity,

                paint_scope: PaintScope::detached(),

                layer: None,

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
        if render_object.opacity != self.opacity {
            let was_layer = render_object.needs_layer();
            render_object.opacity = self.opacity;
            let now_layer = render_object.needs_layer();

            if was_layer && now_layer {
                // Still partial: poke the retained layer's alpha in place; the next composite picks it up.
                if let Some(layer) = &render_object.layer {
                    layer.borrow_mut().set_alpha(self.opacity);
                }
            } else if was_layer == now_layer {
                // Both fully transparent or fully opaque: a visibility flip still repaints.
                render_object.paint_scope.mark_needs_paint();
            } else {
                // Crossing the threshold where a layer is needed changes the compositing bits.
                render_object
                    .paint_scope
                    .mark_needs_compositing_bits_update();
            }
        }

        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

pub struct RenderOpacity<Child> {
    opacity: f32,

    paint_scope: PaintScope,

    /// The layer painted at the last partial-opacity paint, retained so an opacity change that stays
    /// partial can recomposite it at the new alpha without repainting the subtree.
    layer: Option<LayerHandle<OpacityLayer>>,

    child: RenderNode<Child, Option<Size>>,
}

impl<Child> SingleChildRenderObject for RenderOpacity<Child> {
    type Child = Child;

    fn with_child<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
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
        self.paint_scope = ctx.paint_scope().clone();
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
            self.layer = None;
            return;
        }

        if self.opacity >= 1.0 {
            self.layer = None;
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

        // Retain the layer so a later partial-to-partial opacity change can recomposite it in place.
        let layer = LayerHandle::new(OpacityLayer::new(self.opacity, clip));
        self.layer = Some(layer.clone());

        ctx.push_layer(layer, offset, |ctx| self.child.paint(ctx, Offset::ZERO));
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use agui_core::{
        paint::{command::PaintCommand, compositing::ContainerLayer, peniko::Color},
        pipeline::PipelineOwner,
        prelude::{element::*, render_object::*},
        test_harness::with_ctx,
    };

    use crate::{colored_box::ColoredBox, sized_box::SizedBox};

    use super::*;

    /// The compositing bit is property-dependent: a partial opacity needs a layer, the whole/none
    /// fast paths do not.
    #[test]
    fn the_compositing_bit_tracks_the_opacity() {
        for (opacity, needs) in [(0.0, false), (0.5, true), (1.0, false)] {
            let widget = Opacity::new(opacity).child(SizedBox::new().width(10).height(10));
            let (_, mut render) = with_ctx(|ctx| widget.create(ctx));

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
        let (_, render) = with_ctx(|ctx| widget.create(ctx));

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

    /// A leaf that counts its paints, so a test can prove the subtree is not repainted on an opacity tween.
    struct Counter {
        paints: std::rc::Rc<std::cell::Cell<usize>>,
    }

    struct CounterElement;

    impl Element for CounterElement {
        type Render = RenderCounter;
    }

    impl Widget for Counter {
        type Element = CounterElement;
        type Render = RenderCounter;

        fn create(self, _: &mut UpdateCtx) -> (CounterElement, RenderCounter) {
            (
                CounterElement,
                RenderCounter {
                    paints: self.paints,
                },
            )
        }

        fn update(self, _: &mut CounterElement, _: &mut RenderCounter, _: &mut UpdateCtx) {}
    }

    struct RenderCounter {
        paints: std::rc::Rc<std::cell::Cell<usize>>,
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
            None
        }
        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }
        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }
        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
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
            canvas.fill(
                agui_core::paint::peniko::Fill::NonZero,
                brush,
                &(offset & Size::new(10.0, 10.0)),
            );
        }
    }

    /// An opacity change that stays partial pokes the retained layer's alpha in place, without repainting
    /// the subtree.
    #[test]
    fn a_partial_opacity_change_recomposites_without_repainting() {
        use std::cell::Cell;

        use agui_core::{
            context::MountCtx,
            paint::compositing::Compositor,
            pipeline::{layout::BoundaryContent, paint::PaintPipeline},
        };

        let paints = Rc::new(Cell::new(0));
        let widget = Opacity::new(0.5).child(Counter {
            paints: Rc::clone(&paints),
        });
        let (mut element, mut render) = with_ctx(|ctx| widget.create(ctx));

        // Mount under a repaint boundary so the opacity captures a scope it can mark.
        let dummy: BoundaryContent = Rc::new(RefCell::new(RenderCounter {
            paints: Rc::new(Cell::new(0)),
        }));
        let (mut pipeline, boundary) =
            PaintPipeline::new(dummy, LayerHandle::new(ContainerLayer::new()));
        {
            let mut ctx = MountCtx::new(&mut pipeline, boundary.scope());
            render.mount(&mut ctx);
        }
        render.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 100, 0, 100),
        );

        // First paint into a host layer builds and retains the opacity layer.
        let host = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&host, |ctx| render.paint(ctx, Offset::ZERO));
        assert_eq!(paints.get(), 1, "the subtree paints once");

        pipeline.flush_compositing_bits();
        pipeline.flush_paint();

        // A partial-to-partial change pokes the retained layer's alpha in place, with no repaint.
        let next = Opacity::new(0.25).child(Counter {
            paints: Rc::clone(&paints),
        });
        with_ctx(|ctx| next.update(&mut element, &mut render, ctx));

        assert_eq!(paints.get(), 1, "the subtree was not repainted");

        let alpha = Compositor::compose(&host)
            .flatten()
            .commands()
            .iter()
            .find_map(|command| match command {
                PaintCommand::PushLayer { alpha, .. } => Some(*alpha),
                _ => None,
            });
        assert!(
            matches!(alpha, Some(a) if (a - 0.25).abs() < 1e-6),
            "the retained layer recomposited at the new alpha, got {alpha:?}"
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
            .run(|| Opacity::new(0.5).child(SizedBox::new().width(20).height(10)));
    }
}
