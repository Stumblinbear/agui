use std::borrow::Cow;

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Constraints, FontStyle, IntrinsicDimension, Size},
    widget::{
        BuildContext, Children, IntrinsicSizeContext, LayoutContext, PaintContext, WidgetView,
    },
};
use agui_macros::StatelessWidget;

pub mod edit;
pub mod query;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    /// The horizontal line used to align the bottom of glyphs for alphabetic characters.
    Alphabetic,

    /// The horizontal line used to align ideographic characters.
    Ideographic,
}

#[derive(StatelessWidget, Debug, Default, PartialEq)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetView for Text {
    fn intrinsic_size(
        &self,
        _: &mut IntrinsicSizeContext<Self>,
        dimension: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        match dimension {
            // Need to do actual text layout to get the correct intrinsic width
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth => {
                self.text.len() as f32 * self.font.size
            }

            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => self.font.size,
        }
    }

    fn layout(&self, _: &mut LayoutContext<Self>, _: Constraints) -> Size {
        Size {
            width: self.text.len() as f32 * self.font.size,
            height: self.font.size,
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
