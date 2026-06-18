use agui_primitives::{colored_box::ColoredBox, fractionally_sized_box::FractionallySizedBox};
use agui_render::prelude::{
    element::{Alignment, Widget},
    render_object::RenderBox,
};
use agui_test::golden;
use agui_test::golden::VelloHeadless;
use vello::peniko::Color;

#[golden(renderers(VelloHeadless), width = 200, height = 100, tolerance = 0.01)]
fn orange_half_pane() -> impl Widget<Render: RenderBox> {
    // An orange box filling the left half.
    FractionallySizedBox::new()
        .alignment(Alignment::CENTER_LEFT)
        .width_factor(0.5)
        .height_factor(1.0)
        .child(ColoredBox::new(Color::from_rgb8(255, 138, 0)))
}
