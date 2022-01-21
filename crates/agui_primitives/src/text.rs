use agui_core::{
    canvas::font::FontDescriptor,
    unit::{Color, Layout, Margin, Position, Sizing, Units},
    widget::{BuildResult, WidgetBuilder, WidgetContext},
};
use agui_macros::Widget;

#[derive(Clone, PartialEq)]
pub struct TextSection {
    pub font: FontDescriptor,
    pub text: String,
    pub scale: f32,
}

impl TextSection {
    pub fn new(font: FontDescriptor, scale: f32, text: String) -> Self {
        Self { font, text, scale }
    }
}

impl ToSectionText for TextSection {
    fn to_section_text(&self) -> SectionText<'_> {
        SectionText {
            text: &self.text,
            scale: self.scale.into(),
            font_id: FontId(self.font.0),
        }
    }
}

#[derive(Widget)]
pub struct Text {
    pub position: Position,
    pub sizing: Sizing,
    pub max_sizing: Sizing,

    pub wrap: bool,

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

            color: Color::Black,
            sections: Vec::default(),
        }
    }
}

impl WidgetBuilder for Text {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        let sizing = match self.sizing {
            Sizing::Auto => {
                let fonts = ctx.use_global(Fonts::default);
                let fonts = fonts.read();
                let fonts = fonts.get_fonts();

                let glyphs = self.get_glyphs(fonts, (f32::MAX, f32::MAX));

                let mut max_x: f32 = 0.0;
                let mut max_y: f32 = 0.0;

                for g in glyphs {
                    if let Some(font) = fonts.get(g.font_id.0) {
                        max_x += font.as_scaled(g.glyph.scale).h_advance(g.glyph.id);
                        max_y = max_y.max(g.glyph.scale.y);
                    }
                }

                Sizing::Axis {
                    width: Units::Pixels(max_x),
                    height: Units::Pixels(max_y),
                }
            }

            Sizing::Axis { width, height } => {
                if width != Units::Auto && height != Units::Auto {
                    Sizing::Axis { width, height }
                } else {
                    let fonts = ctx.use_global(Fonts::default);
                    let fonts = fonts.read();
                    let fonts = fonts.get_fonts();

                    let glyphs = self.get_glyphs(fonts, (f32::MAX, f32::MAX));

                    let mut max_x: f32 = 0.0;
                    let mut max_y: f32 = 0.0;

                    for g in glyphs {
                        if let Some(font) = fonts.get(g.font_id.0) {
                            max_x += font.as_scaled(g.glyph.scale).h_advance(g.glyph.id);
                            max_y = max_y.max(g.glyph.scale.y);
                        }
                    }

                    Sizing::Axis {
                        width: if width == Units::Auto {
                            Units::Pixels(max_x)
                        } else {
                            width
                        },

                        height: if height == Units::Auto {
                            Units::Pixels(max_y)
                        } else {
                            height
                        },
                    }
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

        BuildResult::None
    }
}

impl Text {
    pub fn is(font: FontDescriptor, scale: f32, text: String) -> Self {
        Self::new(vec![TextSection::new(font, scale, text)])
    }

    pub fn new(sections: Vec<TextSection>) -> Self {
        Self {
            sections,
            ..Text::default()
        }
    }

    pub fn sizing(mut self, sizing: Sizing) -> Self {
        self.sizing = sizing;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn nowrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    pub fn get_glyphs(&self, fonts: &[FontArc], bounds: (f32, f32)) -> Vec<SectionGlyph> {
        let glyphs_layout = GlyphLayout::Wrap {
            line_breaker: BuiltInLineBreaker::UnicodeLineBreaker,
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
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
