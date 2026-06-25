use agui::widgets::{
    colored_box::ColoredBox, rich_text::RichText, sized_box::SizedBox, text::Text,
};
use agui::{
    paint::peniko::Color,
    prelude::{element::Widget, render_object::*},
    provide::Provide,
};
use agui_test::golden;
use agui_test::golden::VelloHeadless;

/// The bundled Cantarell font (SIL Open Font License), so text shapes against a known font rather
/// than whatever the host system provides, keeping the golden deterministic across machines.
const FONT: &[u8] = include_bytes!("fonts/Cantarell-Regular.ttf");

/// A dark background the text reads against, filling the frame so the golden has no transparent gaps.
const BACKGROUND: Color = Color::from_rgb8(30, 30, 30);

#[golden(renderers(VelloHeadless), width = 220, height = 60, tolerance = 0.01)]
fn text_agui() -> impl Widget<Render: RenderBox> {
    // Provide the fonts above the text so its render captures the handle, just as the tree does.
    let fonts = Fonts::new();
    fonts.register(FONT.to_vec());
    Provide::new(fonts).child(
        ColoredBox::new(BACKGROUND).child(
            Text::new("Agui")
                .font_size(40.0)
                .family("Cantarell")
                .brush(Color::WHITE),
        ),
    )
}

#[golden(renderers(VelloHeadless), width = 360, height = 80, tolerance = 0.01)]
fn rich_text_agui() -> impl Widget<Render: RenderBox> {
    let fonts = Fonts::new();
    fonts.register(FONT.to_vec());

    Provide::new(fonts).child(
        ColoredBox::new(BACKGROUND).child(RichText::new(
            TextSpan::new("")
                .style(TextStyle::new().font_size(28.0).family("Cantarell"))
                .children([
                    InlineSpan::Text(
                        TextSpan::new("Bold ").style(
                            TextStyle::new()
                                .color(Color::from_rgb8(255, 80, 80))
                                .weight(FontWeight::BOLD),
                        ),
                    ),
                    InlineSpan::Text(
                        TextSpan::new("under ")
                            .style(TextStyle::new().color(Color::WHITE).underline(true)),
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
                ]),
        )),
    )
}
