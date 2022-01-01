use glyph_brush_layout::{
    BuiltInLineBreaker, GlyphPositioner, SectionGeometry, SectionText, ToSectionText,
};

use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Margin, Position, Sizing},
    widget::{BuildResult, WidgetBuilder},
};
use agui_macros::Widget;

mod font;

pub use self::font::{FontArc, Fonts, GlyphLayout};
pub use glyph_brush_layout::{FontId, HorizontalAlign, SectionGlyph, VerticalAlign};

pub struct TextSection {
    pub font: FontId,
    pub text: String,
    pub scale: f32,
}

impl TextSection {
    pub fn new(font: FontId, scale: f32, text: String) -> Self {
        Self { font, text, scale }
    }
}

impl ToSectionText for TextSection {
    fn to_section_text(&self) -> SectionText<'_> {
        SectionText {
            text: &self.text,
            scale: self.scale.into(),
            font_id: self.font,
        }
    }
}

#[derive(Widget)]
pub struct Text {
    pub position: Position,

    pub sizing: Sizing,
    pub max_sizing: Sizing,

    pub wrap: bool,
    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,

    pub sections: Vec<TextSection>,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            position: Position::default(),

            sizing: Sizing::Fill,
            max_sizing: Sizing::default(),

            wrap: true,
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,

            sections: Vec::default(),
        }
    }
}

impl WidgetBuilder for Text {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout(
            Layout {
                position: self.position,
                min_sizing: Sizing::default(),
                max_sizing: self.max_sizing,
                sizing: self.sizing,
                margin: Margin::default(),
            }
            .into(),
        );

        BuildResult::Empty
    }
}

impl Text {
    pub fn is(font: FontId, scale: f32, text: String) -> Self {
        Self {
            sections: vec![TextSection::new(font, scale, text)],
            ..Text::default()
        }
    }

    pub fn new(sections: Vec<TextSection>) -> Self {
        Self {
            sections,
            ..Text::default()
        }
    }

    pub fn get_glyphs(&self, fonts: &[FontArc], bounds: (f32, f32)) -> Vec<SectionGlyph> {
        let glyphs_layout = GlyphLayout::Wrap {
            line_breaker: BuiltInLineBreaker::UnicodeLineBreaker,
            h_align: self.h_align,
            v_align: self.v_align,
        };

        glyphs_layout.calculate_glyphs(
            fonts,
            &SectionGeometry {
                screen_position: (0.0, 0.0),
                bounds,
            },
            &self.sections,
        )
    }
}
