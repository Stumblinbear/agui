use std::borrow::Cow;

use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Axis, Constraints, FontStyle, IntrinsicDimension, Size},
    widget::{
        BuildContext, ContextWidgetLayoutMut, IntrinsicSizeContext, LayoutContext, WidgetBuild,
        WidgetLayout, WidgetPaint,
    },
};
use agui_macros::{LayoutWidget, PaintWidget};

pub mod edit;
pub mod query;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextBaseline {
    /// The horizontal line used to align the bottom of glyphs for alphabetic characters.
    Alphabetic,

    /// The horizontal line used to align ideographic characters.
    Ideographic,
}

#[derive(LayoutWidget, Debug, Default, PartialEq)]
pub struct Text {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetBuild for Text {
    type Child = TextPainter;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        TextPainter {
            font: self.font.clone(),
            text: Cow::clone(&self.text),
        }
    }
}

impl WidgetLayout for Text {
    fn intrinsic_size(
        &self,
        _: &mut IntrinsicSizeContext<Self>,
        dimension: IntrinsicDimension,
        _: f32,
    ) -> f32 {
        match dimension.axis() {
            // TODO: Actual text layout to get the correct intrinsic width
            Axis::Horizontal => self.text.len() as f32 * self.font.size,
            Axis::Vertical => self.font.size,
        }
    }

    fn layout(&self, ctx: &mut LayoutContext<Self>, _: Constraints) -> Size {
        let size = Size {
            width: self.text.len() as f32 * self.font.size,
            height: self.font.size,
        };

        if let Some(mut child) = ctx.iter_children_mut().next() {
            child.compute_layout(size);
        }

        size
    }
}

#[derive(PaintWidget, Debug, Default, PartialEq)]
pub struct TextPainter {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetPaint for TextPainter {
    fn paint(&self, mut canvas: CanvasPainter) {
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
