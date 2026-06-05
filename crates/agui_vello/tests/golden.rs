//! Golden image test: render a widget tree off-screen and compare it to a reference PNG.
//!
//! The golden is generated on first run (or with `AGUI_UPDATE_GOLDEN=1`) and must be regenerated on
//! the machine the tests run on, since GPU output varies between drivers. The test skips, rather than
//! fails, when no GPU adapter is available.

use agui_core::{
    constraints::Constraints,
    offset::Offset,
    paint::{Compositor, ContainerLayer, LayerHandle, PaintCtx, peniko::Color},
    render_object::{LayoutScope, box_layout::RenderBox},
    test_harness::TestHarness,
    widget::Widget,
};
use agui_primitives::{colored_box::ColoredBox, fractionally_sized_box::FractionallySizedBox};
use agui_vello::{
    headless::{HeadlessRenderer, assert_golden},
    to_vello_scene,
};

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
        .child(ColoredBox::new(Color::rgb8(255, 138, 0)));

    let mut render = widget.create_render_object(&TestHarness::mount(&widget).root.element);
    render.layout(
        &LayoutScope::detached(),
        Constraints::new(0.0, width as f32, 0.0, height as f32),
    );

    let root = LayerHandle::new(ContainerLayer::new());
    PaintCtx::paint(&root, |ctx| render.paint(ctx, Offset::ZERO));
    let scene = Compositor::compose(&root);
    let vello_scene = to_vello_scene(&scene);

    let image = headless.render(&vello_scene, width, height, Color::rgb8(30, 30, 30));

    assert_golden(
        &image,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/goldens/orange_half_pane.png"
        ),
    );
}
