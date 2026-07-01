// These tests shape real text through the platform font backend (DirectWrite on Windows), which Miri
// cannot call, so the suite is excluded under Miri. The harness lives here rather than in a unit module
// because `agui_test` depends on `agui`, and a `#[cfg(test)]` module inside `agui` would link a second,
// distinct copy of `agui` whose `Widget` trait does not match the harness's.
#![cfg(not(miri))]

use std::{rc::Rc, time::Duration};

use agui::{
    paint::{command::PaintCommand, peniko::Color, scene::Scene},
    prelude::render_object::*,
    provide::Provide,
    widgets::rich_text::RichText,
};
use agui_test::{Probe, WidgetTester};

fn with_fonts(content: ParagraphContent) -> RenderParagraph {
    let mut paragraph = RenderParagraph::new(content);
    paragraph.set_fonts(Some(Rc::new(Fonts::new())));
    paragraph
}

/// Builds content with a single styled run over the whole string.
fn styled(text: &str, style: TextStyle) -> ParagraphContent {
    ParagraphContent {
        runs: vec![(0..text.len(), style)],
        text: text.to_owned(),
        placeholders: Vec::new(),
    }
}

/// A single styled run as the span the harness mounts as rich text.
fn text_span(text: &str, style: TextStyle) -> TextSpan {
    TextSpan::<()>::new(text).style(style)
}

/// Lays `span` out as rich text under an ambient font registry, returning the size its paragraph took.
fn laid_out_size(span: TextSpan, constraints: BoxConstraints) -> Size {
    let probe = Probe::new();
    let mut tester =
        WidgetTester::mount(Provide::new(Fonts::new()).child(probe.wrap(RichText::new(span))));
    tester.resize_with(constraints);
    tester.pump(Duration::ZERO);
    probe.size()
}

/// Paints `span` as rich text under an ambient font registry, returning the flattened scene for
/// inspecting the recorded commands.
fn painted_scene(span: TextSpan, constraints: BoxConstraints) -> Scene {
    let mut tester = WidgetTester::mount(Provide::new(Fonts::new()).child(RichText::new(span)));
    tester.resize_with(constraints);
    tester.pump(Duration::ZERO);
    tester.composite_frame().rasterize()
}

fn has_command(scene: &Scene, pred: impl Fn(&PaintCommand) -> bool) -> bool {
    scene.commands().iter().any(pred)
}

#[test]
fn measure_agrees_with_layout_after_shaping() {
    let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);
    let style = TextStyle::new().font_size(20.0);

    let measured = with_fonts(styled("hello world", style.clone())).measure(constraints);
    let laid_out = laid_out_size(text_span("hello world", style), constraints);

    assert_eq!(measured, laid_out);
}

#[test]
fn measure_works_without_a_prior_layout() {
    let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);
    let style = TextStyle::new().font_size(20.0);

    // `measure` shapes on demand, so a paragraph that has never laid out still predicts the size a layout
    // pass produces.
    let measured = with_fonts(styled("no prior layout", style.clone())).measure(constraints);
    let laid_out = laid_out_size(text_span("no prior layout", style), constraints);

    assert_eq!(measured, laid_out);
}

#[test]
fn unbounded_width_layout_agrees_with_measure() {
    let constraints = BoxConstraints::default();
    let style = TextStyle::new().font_size(20.0);

    let measured = with_fonts(styled("hello world", style.clone())).measure(constraints);
    let laid_out = laid_out_size(text_span("hello world", style), constraints);

    assert_eq!(laid_out, measured);
    assert!(laid_out.height.get() > 0.0, "the text occupies a line");
}

#[test]
fn larger_font_run_grows_the_paragraph() {
    let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);

    let small = laid_out_size(
        text_span("ABCDEF", TextStyle::new().font_size(10.0)),
        constraints,
    );
    let large = laid_out_size(
        text_span("ABCDEF", TextStyle::new().font_size(40.0)),
        constraints,
    );

    assert!(large.width > small.width);
    assert!(large.height > small.height);
}

#[test]
fn two_runs_are_wider_than_either_alone() {
    let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
    let style = TextStyle::new().font_size(20.0);

    let one = laid_out_size(text_span("hello", style.clone()), constraints);

    let two_runs = TextSpan::<()>::new("hello")
        .style(style.clone())
        .children([InlineSpan::Text(TextSpan::<()>::new("hello").style(style))]);
    let two = laid_out_size(two_runs, constraints);

    assert!(two.width > one.width);
}

#[test]
fn background_run_paints_a_fill_behind_the_glyphs() {
    let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);

    let highlighted = painted_scene(
        text_span(
            "hi",
            TextStyle::new()
                .font_size(20.0)
                .background(Color::from_rgb8(200, 100, 50)),
        ),
        constraints,
    );
    let plain = painted_scene(
        text_span("hi", TextStyle::new().font_size(20.0)),
        constraints,
    );

    assert!(
        has_command(&highlighted, |command| matches!(
            command,
            PaintCommand::Fill { .. }
        )),
        "a highlighted run records a fill behind its glyphs"
    );
    assert!(
        !has_command(&plain, |command| matches!(
            command,
            PaintCommand::Fill { .. }
        )),
        "the fill belongs to the highlight, not to the surrounding frame"
    );
}

#[test]
fn underline_run_paints_a_stroke() {
    let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);

    let underlined = painted_scene(
        text_span("hi", TextStyle::new().font_size(20.0).underline(true)),
        constraints,
    );
    let plain = painted_scene(
        text_span("hi", TextStyle::new().font_size(20.0)),
        constraints,
    );

    assert!(
        has_command(&underlined, |command| matches!(
            command,
            PaintCommand::Stroke { .. }
        )),
        "an underlined run records a stroke"
    );
    assert!(
        !has_command(&plain, |command| matches!(
            command,
            PaintCommand::Stroke { .. }
        )),
        "the stroke belongs to the underline, not to the surrounding frame"
    );
}

#[test]
fn narrower_break_does_not_grow_width() {
    let text = "the quick brown fox jumps over the lazy dog";
    let style = TextStyle::new().font_size(20.0);

    let wide_width = laid_out_size(
        text_span(text, style.clone()),
        BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0),
    )
    .width;
    let narrow_width = laid_out_size(
        text_span(text, style),
        BoxConstraints::new(0.0, 80.0, 0.0, 1000.0),
    )
    .width;

    assert!(narrow_width <= wide_width);
}
