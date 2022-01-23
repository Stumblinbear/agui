use std::borrow::Cow;

use agui_core::{
    canvas::{font::FontStyle, paint::Paint, painter::CanvasPainter, Canvas},
    unit::{Layout, Ref},
    widget::{BuildResult, WidgetBuilder, WidgetContext},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Text {
    pub layout: Ref<Layout>,

    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl WidgetBuilder for Text {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        ctx.set_layout(Ref::clone(&self.layout));

        BuildResult::None
    }
}

pub struct TextPainter {
    pub font: FontStyle,
    pub text: Cow<'static, str>,
}

impl CanvasPainter for TextPainter {
    fn draw(&self, canvas: &mut Canvas) {
        let brush = canvas.new_brush(Paint {
            color: self.font.color,
        });

        canvas.draw_text(brush, self.font.font_id, Cow::clone(&self.text));
    }
}

impl Text {
    // pub fn get_glyphs(&self, fonts: &[FontArc], bounds: (f32, f32)) -> Vec<SectionGlyph> {
    //     let glyphs_layout = GlyphLayout::Wrap {
    //         line_breaker: BuiltInLineBreaker::UnicodeLineBreaker,
    //         h_align: HorizontalAlign::Left,
    //         v_align: VerticalAlign::Top,
    //     };

    //     glyphs_layout.calculate_glyphs(
    //         fonts,
    //         &SectionGeometry {
    //             screen_position: (0.0, 0.0),
    //             bounds,
    //         },
    //         &self.sections,
    //     )
    // }
}
