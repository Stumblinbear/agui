use std::borrow::Cow;

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{FontStyle, Layout, LayoutType, Sizing},
    widget::{BuildContext, Children, LayoutContext, LayoutResult, PaintContext, WidgetView},
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

    fn build(&self, _ctx: &mut BuildContext<Self>) -> Children {
        Children::none()
    }

    fn paint(&self, _ctx: &mut PaintContext<Self>, mut canvas: CanvasPainter) {
        canvas.draw_text(
            &Paint {
                color: self.font.color,
                ..Paint::default()
            },
            self.font.clone(),
            Cow::clone(&self.text),
        );
    }
}
