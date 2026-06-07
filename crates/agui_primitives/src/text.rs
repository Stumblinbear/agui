use std::rc::Rc;

use agui_core::prelude::{element::*, render_object::*};

/// A leaf widget that shapes and sizes a run of text.
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
}

pub struct TextElement {
    fonts: Option<Rc<Fonts>>,
}

impl Element for TextElement {}

impl Widget for Text {
    type Element = TextElement;

    type Render = RenderParagraph;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        TextElement {
            fonts: ctx.get_provided::<Fonts>(),
        }
    }

    fn update(&self, element: &mut Self::Element, _: &Self, ctx: &mut UpdateCtx) {
        element.fonts = ctx.get_provided::<Fonts>();
    }

    fn dispatch(&self, _: &mut Self::Element, _: &[RoutingId], _: Dispatch) {}

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        let mut paragraph = RenderParagraph::new(self.text.clone());
        paragraph.set_font_size(self.font_size);
        paragraph.set_brush(self.brush.clone());
        paragraph.set_font_family(self.family.clone());
        paragraph.set_fonts(element.fonts.clone());
        paragraph
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        render_object.set_text(self.text.clone());
        render_object.set_font_size(self.font_size);
        render_object.set_brush(self.brush.clone());
        render_object.set_font_family(self.family.clone());
        render_object.set_fonts(element.fonts.clone());
    }
}

#[cfg(test)]
mod harness {
    use std::rc::Rc;

    use agui_core::prelude::render_object::Fonts;
    use agui_test::prelude::*;

    use super::Text;
    use crate::provide::Provide;

    #[test]
    fn produces_a_finite_size() {
        let probe = Probe::new();
        let fonts = Rc::new(Fonts::new());
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
