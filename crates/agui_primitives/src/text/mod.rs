use std::borrow::Cow;

use agui_core::prelude::*;

pub mod edit;
pub mod query;

#[derive(Debug, Default)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
    pub multiline: bool,
}

impl StatelessWidget for Text {
    fn build(&self, ctx: &mut BuildContext<()>) -> BuildResult {
        ctx.set_layout(Layout {
            sizing: if self.multiline {
                Sizing::Fill
            } else {
                Sizing::Axis {
                    width: Units::Stretch(1.0),
                    height: Units::Pixels(self.font.size),
                }
            },
            ..Layout::default()
        });

        ctx.on_draw({
            let font = self.font.clone();
            let text = self.text.clone();

            move |canvas| {
                let brush = canvas.new_brush(Paint {
                    color: font.color,
                    ..Paint::default()
                });

                canvas.draw_text(brush, font.clone(), Cow::clone(&text));
            }
        });

        BuildResult::None
    }
}
