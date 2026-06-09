use std::{borrow::Cow, ops::Range};

use parley::{
    FontFamily, FontFamilyName, RangedBuilder, StyleProperty,
    style::{FontStyle, FontWeight, FontWidth, LineHeight},
};
use peniko::Brush;

use crate::text::TextBrush;

/// A set of text attributes applied to a span, with every attribute optional.
///
/// An unset attribute is inherited from the enclosing span, so a style names only what it changes
/// and the rest cascades in. [`color`](Self::color) and [`background`](Self::background) inherit
/// independently: a span may highlight text without disturbing the color it inherits.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct TextStyle {
    /// The size of the font, in logical pixels.
    pub font_size: Option<f32>,
    /// The fill painted on the glyphs.
    pub color: Option<Brush>,
    /// A fill painted as a solid block behind the text.
    pub background: Option<Brush>,
    /// The family the font is selected from.
    pub family: Option<String>,
    /// The thickness of the font's strokes.
    pub weight: Option<FontWeight>,
    /// Whether the font is upright, italic, or oblique.
    pub style: Option<FontStyle>,
    /// The width of the font's glyphs.
    pub width: Option<FontWidth>,
    /// Whether a line is drawn beneath the text.
    pub underline: Option<bool>,
    /// Whether a line is drawn through the text.
    pub strikethrough: Option<bool>,
    /// Extra space added between letters, in logical pixels.
    pub letter_spacing: Option<f32>,
    /// Extra space added between words, in logical pixels.
    pub word_spacing: Option<f32>,
    /// The height of each line the text occupies.
    pub line_height: Option<LineHeight>,
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(font_size);
        self
    }

    pub fn color(mut self, color: impl Into<Brush>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn background(mut self, background: impl Into<Brush>) -> Self {
        self.background = Some(background.into());
        self
    }

    pub fn family(mut self, family: impl Into<String>) -> Self {
        self.family = Some(family.into());
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn width(mut self, width: FontWidth) -> Self {
        self.width = Some(width);
        self
    }

    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = Some(underline);
        self
    }

    pub fn strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = Some(strikethrough);
        self
    }

    pub fn letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing = Some(letter_spacing);
        self
    }

    pub fn word_spacing(mut self, word_spacing: f32) -> Self {
        self.word_spacing = Some(word_spacing);
        self
    }

    pub fn line_height(mut self, line_height: LineHeight) -> Self {
        self.line_height = Some(line_height);
        self
    }

    /// Returns the result of layering this style over `base`, with each set attribute here taking
    /// precedence and each unset one falling through to `base`.
    pub(crate) fn overlay(&self, base: &TextStyle) -> TextStyle {
        TextStyle {
            font_size: self.font_size.or(base.font_size),
            color: self.color.clone().or_else(|| base.color.clone()),
            background: self.background.clone().or_else(|| base.background.clone()),
            family: self.family.clone().or_else(|| base.family.clone()),
            weight: self.weight.or(base.weight),
            style: self.style.or(base.style),
            width: self.width.or(base.width),
            underline: self.underline.or(base.underline),
            strikethrough: self.strikethrough.or(base.strikethrough),
            letter_spacing: self.letter_spacing.or(base.letter_spacing),
            word_spacing: self.word_spacing.or(base.word_spacing),
            line_height: self.line_height.or(base.line_height),
        }
    }

    /// Pushes each set attribute onto `builder` over `range`, baking [`color`](Self::color) and
    /// [`background`](Self::background) together into the run's brush.
    pub(crate) fn push_into(
        &self,
        builder: &mut RangedBuilder<'_, TextBrush>,
        range: Range<usize>,
    ) {
        if let Some(font_size) = self.font_size {
            builder.push(StyleProperty::FontSize(font_size), range.clone());
        }

        if self.color.is_some() || self.background.is_some() {
            // By this point the cascade is resolved, so an unset color means none was set anywhere.
            let brush = TextBrush {
                fill: self
                    .color
                    .clone()
                    .unwrap_or_else(|| TextBrush::default().fill),
                background: self.background.clone(),
            };
            builder.push(StyleProperty::Brush(brush), range.clone());
        }

        if let Some(family) = &self.family {
            builder.push(
                StyleProperty::FontFamily(FontFamily::Single(FontFamilyName::Named(Cow::Owned(
                    family.clone(),
                )))),
                range.clone(),
            );
        }

        if let Some(weight) = self.weight {
            builder.push(StyleProperty::FontWeight(weight), range.clone());
        }

        if let Some(style) = self.style {
            builder.push(StyleProperty::FontStyle(style), range.clone());
        }

        if let Some(width) = self.width {
            builder.push(StyleProperty::FontWidth(width), range.clone());
        }

        if let Some(underline) = self.underline {
            builder.push(StyleProperty::Underline(underline), range.clone());
        }

        if let Some(strikethrough) = self.strikethrough {
            builder.push(StyleProperty::Strikethrough(strikethrough), range.clone());
        }

        if let Some(letter_spacing) = self.letter_spacing {
            builder.push(StyleProperty::LetterSpacing(letter_spacing), range.clone());
        }

        if let Some(word_spacing) = self.word_spacing {
            builder.push(StyleProperty::WordSpacing(word_spacing), range.clone());
        }

        if let Some(line_height) = self.line_height {
            builder.push(StyleProperty::LineHeight(line_height), range.clone());
        }
    }
}
