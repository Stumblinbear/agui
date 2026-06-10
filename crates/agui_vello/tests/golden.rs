use agui_core::{
    paint::compositing::{Compositor, LayerHandle, OffsetLayer},
    prelude::{element::*, render_object::*},
    test_harness::with_ctx,
};
use agui_primitives::{colored_box::ColoredBox, fractionally_sized_box::FractionallySizedBox};
use agui_vello::{
    headless::{HeadlessRenderer, assert_golden},
    to_vello_scene,
};
use vello::peniko::Color;

#[test]
fn orange_half_pane_matches_golden() {
    let Some(mut headless) = HeadlessRenderer::new() else {
        eprintln!("no GPU adapter available; skipping golden test");
        return;
    };

    let (width, height) = (200_u32, 100_u32);

    // An orange box filling the left half, painted straight into a scene.
    let widget = FractionallySizedBox::new()
        .width_factor(0.5)
        .height_factor(1.0)
        .child(ColoredBox::new(Color::from_rgb8(255, 138, 0)));

    let (_, mut render) = with_ctx(|ctx| widget.create(ctx));
    render.layout(
        &mut LayoutCtx::detached(),
        BoxConstraints::new(0.0, width as f32, 0.0, height as f32),
    );

    let root = LayerHandle::new(OffsetLayer::new());
    PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
    let scene = Compositor::compose(&root);
    let vello_scene = to_vello_scene(&scene);

    let image = headless.render(&vello_scene, width, height, Color::from_rgb8(30, 30, 30));

    assert_golden(
        &image,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/goldens/orange_half_pane.png"
        ),
    );
}
