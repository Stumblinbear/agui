use std::borrow::Cow;

use agui_core::{
    render::canvas::paint::Paint,
    unit::{FontStyle, Layout, LayoutType, Sizing},
    widget::{BuildContext, BuildResult, LayoutContext, LayoutResult, WidgetView},
};
use agui_macros::StatelessWidget;

pub mod edit;
pub mod query;

#[derive(StatelessWidget, Debug, Default, PartialEq)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetView for Text {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: Sizing::Fill,
                min_sizing: Sizing::Axis {
                    width: 0.0.into(),
                    height: self.font.size.into(),
                },
                ..Layout::default()
            },
        }
    }

    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.on_draw(|ctx, mut canvas| {
            canvas.draw_text(
                &Paint {
                    color: ctx.font.color,
                    ..Paint::default()
                },
                ctx.font.clone(),
                Cow::clone(&ctx.text),
            );
        });

        BuildResult::empty()
    }
}
