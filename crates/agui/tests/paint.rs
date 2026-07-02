// Paint-context drawing and layering, driven through the real pipeline: a probe render object runs the
// drawing under test during an actual paint, and the composited scene is inspected. The harness lives here
// rather than a unit module because `agui_test` depends on `agui`.

use std::rc::Rc;
use std::time::Duration;

use agui::{
    context::{CreateCtx, LayoutCtx, PaintCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::LeafElement,
    geometry::{Offset, Rect, Size},
    input::hit_test::{HitTest, HitTestResult},
    paint::{
        Canvas,
        command::PaintCommand,
        compositing::{ContainerLayer, LayerHandle, OffsetLayer, PictureLayer},
        peniko::{Color, Fill, kurbo::Affine},
        scene::Scene,
    },
    render_object::{
        RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    semantics::SemanticsTreeBuilder,
    text::TextBaseline,
    widget::Widget,
};
use agui_test::WidgetTester;
use typed_floats::{Positive, PositiveFinite};

type PaintFn = Rc<dyn Fn(&mut PaintCtx)>;

/// A leaf widget whose render object runs `paint` during the paint pass, so a test can drive `PaintCtx`
/// through the real pipeline.
struct PaintProbe {
    paint: PaintFn,
}

impl PaintProbe {
    fn new(paint: impl Fn(&mut PaintCtx) + 'static) -> Self {
        Self {
            paint: Rc::new(paint),
        }
    }
}

impl Widget for PaintProbe {
    type Element = LeafElement<RenderPaintProbe>;

    type Render = RenderPaintProbe;

    fn create(self, _: &mut CreateCtx) -> LeafElement<RenderPaintProbe> {
        LeafElement::new(RenderPaintProbe { paint: self.paint })
    }

    fn update(self, _: &mut UpdateCtx, element: &mut LeafElement<RenderPaintProbe>) {
        element.render_object_mut().paint = self.paint;
    }
}

struct RenderPaintProbe {
    paint: PaintFn,
}

impl RenderObject for RenderPaintProbe {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

impl RenderBox for RenderPaintProbe {
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

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.biggest()
    }

    fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        constraints.biggest()
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }

    fn update_compositing_bits(&mut self) -> bool {
        false
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _: Offset) {
        (self.paint)(ctx);
    }

    fn build_semantics(&mut self, _: &mut SemanticsTreeBuilder<'_>) {}
}

/// The composited scene from painting `build` through the pipeline.
fn painted(build: impl Fn(&mut PaintCtx) + 'static) -> Scene {
    let mut tester = WidgetTester::mount(PaintProbe::new(build));
    tester.resize_with(BoxConstraints::tight(Size::new(20.0, 20.0)));
    tester.pump(Duration::ZERO);
    tester.composite_frame().rasterize()
}

/// Draws a unit fill at the paint origin.
fn fill(ctx: &mut PaintCtx) {
    let mut canvas = ctx.canvas();
    let brush = canvas.brush(Color::BLACK);
    canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
}

/// A pre-built retained layer holding a single fill, for [`add_layer`](PaintCtx::add_layer).
fn fill_layer() -> LayerHandle<OffsetLayer> {
    let picture = Canvas::record(|canvas| {
        let brush = canvas.brush(Color::BLACK);
        canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(1.0, 1.0)));
    });

    let mut layer = OffsetLayer::new();
    layer.append(LayerHandle::new(PictureLayer::new(picture)).into());
    LayerHandle::new(layer)
}

/// The transform in effect at each fill of the composed, flattened scene, in order.
fn fill_transforms(scene: &Scene) -> Vec<Affine> {
    let mut current = Affine::IDENTITY;
    let mut stack = Vec::new();
    let mut transforms = Vec::new();
    for command in scene.commands() {
        match command {
            PaintCommand::PushTransform(t) => {
                stack.push(current);
                current *= *t;
            }
            PaintCommand::PopTransform => current = stack.pop().expect("balanced"),
            PaintCommand::Fill { .. } => transforms.push(current),
            _ => {}
        }
    }
    transforms
}

#[test]
fn flat_drawing_seals_into_one_picture() {
    assert_eq!(fill_transforms(&painted(fill)), vec![Affine::IDENTITY]);
}

#[test]
fn a_flat_transform_places_drawing_under_it() {
    let scene = painted(|ctx| ctx.with_transform(false, Affine::translate((5.0, 7.0)), fill));
    assert_eq!(fill_transforms(&scene), vec![Affine::translate((5.0, 7.0))]);
}

#[test]
fn a_pushed_layer_carries_its_content() {
    let scene =
        painted(|ctx| ctx.push_layer(LayerHandle::new(OffsetLayer::new()), Offset::ZERO, fill));
    assert_eq!(fill_transforms(&scene), vec![Affine::IDENTITY]);
}

#[test]
fn a_layer_added_at_an_offset_is_positioned_there() {
    let scene = painted(|ctx| ctx.add_layer(fill_layer(), Offset::new(3.0, 0.0)));
    assert_eq!(fill_transforms(&scene), vec![Affine::translate((3.0, 0.0))]);
}

#[test]
fn drawing_resumes_under_the_same_bracket_after_a_layer() {
    let scene = painted(|ctx| {
        ctx.with_transform(true, Affine::translate((2.0, 0.0)), |ctx| {
            fill(ctx);
            ctx.add_layer(fill_layer(), Offset::ZERO);
            fill(ctx);
        });
    });

    assert_eq!(
        fill_transforms(&scene),
        vec![
            Affine::translate((2.0, 0.0)),
            Affine::translate((2.0, 0.0)),
            Affine::translate((2.0, 0.0)),
        ]
    );
}

/// A layer sealed inside a pushed layer and a bracketed transform reproduce the same flattened content on a
/// fresh paint of identical content.
#[test]
fn the_same_content_paints_identically() {
    fn content(ctx: &mut PaintCtx) {
        fill(ctx);
        ctx.push_layer(LayerHandle::new(OffsetLayer::new()), Offset::ZERO, fill);
        ctx.with_transform(false, Affine::translate((5.0, 0.0)), fill);
    }

    assert_eq!(
        fill_transforms(&painted(content)),
        fill_transforms(&painted(content))
    );
}
