use std::{cell::RefCell, rc::Rc};

use agui_core::{
    paint::compositing::{ContainerLayer, LayerHandle},
    pipeline::{PipelineOwner, layout::BoundaryContent},
    prelude::{element::*, render_object::*},
    test_harness::TestHarness,
};
use agui_primitives::{provide::Provide, text::Text};
use agui_vello::{
    headless::{HeadlessRenderer, assert_golden},
    to_vello_scene,
};
use vello::peniko::Color;

/// The bundled Cantarell font (SIL Open Font License), so text shapes against a known font rather
/// than whatever the host system provides, keeping the golden deterministic across machines.
const FONT: &[u8] = include_bytes!("fonts/Cantarell-Regular.ttf");

#[test]
fn text_renders_glyphs_matching_golden() {
    let Some(mut headless) = HeadlessRenderer::new() else {
        eprintln!("no GPU adapter available; skipping golden test");
        return;
    };

    let (width, height) = (220_u32, 60_u32);

    // White text on the dark base; the registered family forces the bundled font.
    let text = || {
        Text::new("Agui")
            .font_size(40.0)
            .family("Cantarell")
            .brush(Color::WHITE)
    };

    // Provide the fonts above the text so its element captures the handle, just as the tree does.
    let fonts = Rc::new(Fonts::new());
    fonts.register(FONT.to_vec());
    let widget = Provide::new(fonts).child(text());

    // The Provide wraps the Text; reach the captured Text element to build its render object.
    let harness = TestHarness::mount(&widget);
    let render = text().create_render_object(&harness.root.element.child.element);
    let content: BoundaryContent = Rc::new(RefCell::new(render));

    let layer = LayerHandle::new(ContainerLayer::new());
    let mut owner = PipelineOwner::new(Rc::clone(&content), layer);

    owner.resize(BoxConstraints::new(0.0, width as f32, 0.0, height as f32));
    owner.flush_layout();
    owner.flush_paint();

    let scene = owner.composite();
    let vello_scene = to_vello_scene(&scene);

    let image = headless.render(&vello_scene, width, height, Color::from_rgb8(30, 30, 30));

    assert_golden(
        &image,
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/goldens/text_agui.png"),
    );
}
