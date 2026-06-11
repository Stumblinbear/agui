use std::{cell::RefCell, rc::Rc};

use agui_core::{
    paint::compositing::{LayerHandle, OffsetLayer},
    pipeline::{PipelineOwner, layout::BoundaryContent},
    prelude::{element::*, render_object::*},
    provide::Provide,
    test_harness::with_ctx,
};
use agui_primitives::{
    colored_box::ColoredBox, rich_text::RichText, sized_box::SizedBox, text::Text,
};
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
    let text = Text::new("Agui")
        .font_size(40.0)
        .family("Cantarell")
        .brush(Color::WHITE);

    // Provide the fonts above the text so its render captures the handle, just as the tree does.
    let fonts = Rc::new(Fonts::new());
    fonts.register(FONT.to_vec());
    let widget = Provide::new(fonts).child(text);

    let (_, render) = with_ctx(|ctx| widget.create(ctx));
    let content: BoundaryContent = Rc::new(RefCell::new(render));

    let layer = LayerHandle::new(OffsetLayer::new());
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

#[test]
fn rich_text_renders_styled_runs_matching_golden() {
    let Some(mut headless) = HeadlessRenderer::new() else {
        eprintln!("no GPU adapter available; skipping golden test");
        return;
    };

    let (width, height) = (360_u32, 80_u32);

    // A base style every run inherits, with each child overriding only what it changes: a bold red
    // run, an underlined run, a highlighted run, and an inline colored box.
    let base = TextStyle::new().font_size(28.0).family("Cantarell");
    let span = TextSpan::new("").style(base).children([
        InlineSpan::Text(
            TextSpan::new("Bold ").style(
                TextStyle::new()
                    .color(Color::from_rgb8(255, 80, 80))
                    .weight(FontWeight::BOLD),
            ),
        ),
        InlineSpan::Text(
            TextSpan::new("under ").style(TextStyle::new().color(Color::WHITE).underline(true)),
        ),
        InlineSpan::Text(
            TextSpan::new("mark ").style(
                TextStyle::new()
                    .color(Color::BLACK)
                    .background(Color::from_rgb8(255, 235, 59)),
            ),
        ),
        InlineSpan::Widget(
            ColoredBox::new(Color::from_rgb8(80, 200, 120))
                .child(SizedBox::new().width(30).height(30)),
        ),
    ]);

    let fonts = Rc::new(Fonts::new());
    fonts.register(FONT.to_vec());
    let widget = Provide::new(fonts).child(RichText::new(span));

    let (_, render) = with_ctx(|ctx| widget.create(ctx));
    let content: BoundaryContent = Rc::new(RefCell::new(render));

    let layer = LayerHandle::new(OffsetLayer::new());
    let mut owner = PipelineOwner::new(Rc::clone(&content), layer);

    owner.resize(BoxConstraints::new(0.0, width as f32, 0.0, height as f32));
    owner.flush_layout();
    owner.flush_paint();

    let scene = owner.composite();
    let vello_scene = to_vello_scene(&scene);

    let image = headless.render(&vello_scene, width, height, Color::from_rgb8(30, 30, 30));

    assert_golden(
        &image,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/goldens/rich_text_agui.png"
        ),
    );
}
