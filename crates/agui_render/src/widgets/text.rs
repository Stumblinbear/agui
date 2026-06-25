use std::rc::Rc;

use crate::{
    prelude::{element::*, render_object::*},
    stateless::{StatelessElement, StatelessWidget},
};

/// A leaf widget that shapes and sizes a single styled run of text.
///
/// It reads the ambient [`Fonts`] from scope, so wrap it (directly or above) in a
/// [`Provide`](crate::provide::Provide) of a `Fonts` for it to shape against.
pub struct Text {
    text: String,
    font_size: f32,
    brush: TextBrush,
    family: Option<String>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 16.0,
            brush: TextBrush::default(),
            family: None,
        }
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn brush(mut self, brush: impl Into<TextBrush>) -> Self {
        self.brush = brush.into();
        self
    }

    pub fn family(mut self, family: impl Into<String>) -> Self {
        self.family = Some(family.into());
        self
    }

    fn content(
        text: &str,
        font_size: f32,
        brush: &TextBrush,
        family: Option<&str>,
    ) -> ParagraphContent {
        let mut style = TextStyle::new()
            .font_size(font_size)
            .color(brush.fill.clone());

        if let Some(background) = &brush.background {
            style = style.background(background.clone());
        }

        if let Some(family) = family {
            style = style.family(family);
        }

        TextSpan::<()>::new(text).style(style).flatten().0
    }
}

impl Widget for Text {
    type Element = StatelessElement<Text>;

    type Render = RenderText;

    fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
        StatelessElement::new(self)
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.update_widget(ctx, self);
    }
}

impl StatelessWidget for Text {
    type Child = RawText;

    fn build(&self, ctx: &mut BuildCtx) -> RawText {
        RawText {
            content: Text::content(
                &self.text,
                self.font_size,
                &self.brush,
                self.family.as_deref(),
            ),
            fonts: ctx.depend_on_provided::<Fonts>(),
        }
    }
}

/// A leaf widget that renders pre-built paragraph content into a [`RenderText`]. Its fonts are supplied by
/// the parent that resolved them; `RawText` reads no provided values itself.
pub struct RawText {
    content: ParagraphContent,
    fonts: Option<Rc<Fonts>>,
}

impl Widget for RawText {
    type Element = LeafElement<RenderText>;

    type Render = RenderText;

    fn create(self, _ctx: &mut CreateCtx) -> Self::Element {
        let mut render = RenderText::new(self.content);
        render.set_fonts(self.fonts);

        LeafElement::new(render)
    }

    fn update(self, _ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        render.set_content(self.content);
        render.set_fonts(self.fonts);
    }
}
