use std::rc::Rc;

use agui_core::prelude::{element::*, render_object::*};

/// A leaf widget that shapes and sizes a single styled run of text.
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

pub struct TextElement {
    fonts: Option<Rc<Fonts>>,
    text: String,
    font_size: f32,
    brush: TextBrush,
    family: Option<String>,
}

impl Element for TextElement {
    type Render = RenderParagraph;
}

impl Widget for Text {
    type Element = TextElement;

    type Render = RenderParagraph;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let element = TextElement {
            fonts: ctx.depend_on_provided::<Fonts>(),
            text: self.text,
            font_size: self.font_size,
            brush: self.brush,
            family: self.family,
        };

        let mut paragraph = RenderParagraph::new(Self::content(
            &element.text,
            element.font_size,
            &element.brush,
            element.family.as_deref(),
        ));

        paragraph.set_fonts(element.fonts.clone());

        (element, paragraph)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        element.fonts = ctx.depend_on_provided::<Fonts>();

        if element.text != self.text
            || element.font_size != self.font_size
            || element.brush != self.brush
            || element.family != self.family
        {
            element.text = self.text;
            element.font_size = self.font_size;
            element.brush = self.brush;
            element.family = self.family;

            render_object.set_content(Self::content(
                &element.text,
                element.font_size,
                &element.brush,
                element.family.as_deref(),
            ));
        }

        render_object.set_fonts(element.fonts.clone());
    }
}

#[cfg(test)]
mod harness {
    use agui_core::prelude::render_object::Fonts;
    use agui_test::prelude::*;

    use agui_core::provide::Provide;

    use super::Text;

    #[test]
    fn produces_a_finite_size() {
        let probe = Probe::new();
        let fonts = Fonts::new();

        let mut tester = WidgetTester::mount(
            probe.wrap(Provide::new(fonts).child(Text::new("hello").font_size(20.0))),
        );

        tester.resize_with(BoxConstraints::loose(Size::new(300, 300)));
        tester.pump(Duration::ZERO);

        let size = probe.size();
        assert!(size.width.get().is_finite());
        assert!(size.height.get().is_finite());
    }
}
