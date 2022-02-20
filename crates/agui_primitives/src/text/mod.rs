use std::borrow::Cow;

use agui_core::{
    canvas::paint::Paint,
    font::FontStyle,
    unit::{Layout, Sizing, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder},
};
use agui_macros::Widget;

pub mod edit;

#[derive(Default, Widget)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
    pub multiline: bool,
}

impl WidgetBuilder for Text {
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        ctx.set_layout(
            Layout {
                sizing: if self.multiline {
                    Sizing::Fill
                } else {
                    Sizing::Axis {
                        width: Units::Stretch(1.0),
                        height: Units::Pixels(self.font.size),
                    }
                },
                ..Layout::default()
            }
            .into(),
        );

        ctx.on_draw({
            let font = self.font.clone();
            let text = self.text.clone();

            move |canvas| {
                let brush = canvas.new_brush(Paint { color: font.color });

                canvas.draw_text(brush, font.clone(), Cow::clone(&text));
            }
        });

        BuildResult::None
    }
}
