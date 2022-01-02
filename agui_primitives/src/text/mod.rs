use glyph_brush_layout::{
    BuiltInLineBreaker, GlyphPositioner, SectionGeometry, SectionText, ToSectionText,
};

use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Color, Margin, Position, Sizing, Units},
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

    pub color: Color,
    pub sections: Vec<TextSection>,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            position: Position::default(),

            sizing: Sizing::default(),
            max_sizing: Sizing::default(),

            wrap: true,
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,

            color: Color::Black,
            sections: Vec::default(),
        }
    }
}

impl WidgetBuilder for Text {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        let sizing = match self.sizing {
            Sizing::Auto => {
                let fonts = ctx.use_global(Fonts::default);
                let fonts = fonts.read();

                let glyphs = self.get_glyphs(fonts.get_fonts(), (f32::MAX, f32::MAX));

                let mut max_x: f32 = 0.0;
                let mut max_y: f32 = 0.0;

                for g in glyphs {
                    max_x = max_x.max(g.glyph.position.x + g.glyph.scale.x);
                    max_y = max_y.max(g.glyph.position.y);
                }

                Sizing::Axis {
                    width: Units::Pixels(max_x),
                    height: Units::Pixels(max_y),
                }
            }
            sizing => sizing,
        };

        ctx.set_layout(
            Layout {
                position: self.position,
                min_sizing: Sizing::default(),
                max_sizing: self.max_sizing,
                sizing,
                margin: Margin::default(),
            }
            .into(),
        );

        BuildResult::Empty
    }
}

impl Text {
    pub fn is(font: FontId, color: Color, scale: f32, text: String) -> Self {
        Self {
            color,
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
