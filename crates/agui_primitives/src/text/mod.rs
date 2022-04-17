use std::borrow::Cow;

use agui_core::prelude::*;

pub mod edit;
pub mod query;

#[derive(Debug, Default)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl StatelessWidget for Text {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.set_layout(Layout {
            sizing: Sizing::Fill,
            ..Layout::default()
        });

        ctx.on_draw(|ctx, canvas| {
            let brush = canvas.new_brush(Paint {
                color: ctx.font.color,
                ..Paint::default()
            });

            canvas.draw_text(brush, ctx.font.clone(), Cow::clone(&ctx.text));
        });

        BuildResult::None
    }
}
