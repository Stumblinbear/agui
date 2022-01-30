use std::borrow::Cow;

use agui_core::{
    canvas::{font::FontStyle, paint::Paint},
    unit::{Layout, Sizing},
    widget::{BuildResult, WidgetBuilder, WidgetContext},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetBuilder for Text {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        ctx.set_layout(
            Layout {
                sizing: Sizing::Fill,
                ..Layout::default()
            }
            .into(),
        );

        ctx.on_draw({
            let font = self.font;
            let text = self.text.clone();

            move |canvas| {
                let brush = canvas.new_brush(Paint { color: font.color });

                canvas.draw_text(brush, font, Cow::clone(&text));
            }
        });

        BuildResult::None
    }
}
