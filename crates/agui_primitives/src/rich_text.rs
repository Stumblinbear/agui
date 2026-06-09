use std::rc::Rc;

use agui_core::prelude::{element::*, render_object::*};

/// A widget that lays out a tree of styled spans, with inline widgets flowing alongside the text.
pub struct RichText<Children = ()> {
    span: TextSpan<Children>,
}

impl<Children> RichText<Children> {
    pub fn new(span: impl Into<TextSpan<Children>>) -> Self {
        Self { span: span.into() }
    }
}

/// The element of a [`RichText`].
pub struct RichTextElement<C: Element, R> {
    fonts: Option<Rc<Fonts>>,
    children: MultiChildElement<C, R>,
}

impl<C, R> Element for RichTextElement<C, R>
where
    C: Element,
    R: MultiChildRenderObject<Child = C::Render>,
{
    type Render = R;

    fn dispatch(&mut self, render: &mut R, path: &[RoutingId], action: Dispatch) {
        self.children.dispatch(render, path, action);
    }
}

impl<Children> Widget for RichText<Children>
where
    Children: Widget + 'static,
    Children::Render: RenderObject,
{
    type Element = RichTextElement<Children::Element, RenderParagraph<Children::Render>>;

    type Render = RenderParagraph<Children::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (content, widgets) = self.span.flatten();

        let mut render_object = RenderParagraph::new(content);

        let fonts = ctx.get_provided::<Fonts>();
        render_object.set_fonts(fonts.clone());

        let children = MultiChildElement::new(widgets, &mut render_object, ctx);

        (RichTextElement { fonts, children }, render_object)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        let (content, widgets) = self.span.flatten();

        render_object.set_content(content);

        element.fonts = ctx.get_provided::<Fonts>();
        render_object.set_fonts(element.fonts.clone());

        element.children.update(widgets, render_object, ctx);
    }
}

#[cfg(test)]
mod harness {
    use std::rc::Rc;

    use agui_core::prelude::render_object::{Fonts, InlineSpan, TextSpan, TextStyle};
    use agui_test::prelude::*;

    use super::RichText;
    use crate::{provide::Provide, sized_box::SizedBox};

    #[test]
    fn mixed_styles_produce_a_finite_size() {
        let probe = Probe::new();
        let fonts = Rc::new(Fonts::new());

        let span = TextSpan::new("hello ").children([InlineSpan::Text(
            TextSpan::<()>::new("world").style(TextStyle::new().font_size(28.0)),
        )]);

        let mut tester =
            WidgetTester::mount(probe.wrap(Provide::new(fonts).child(RichText::new(span))));
        tester.resize_with(BoxConstraints::loose(Size::new(300, 300)));
        tester.pump(Duration::ZERO);

        let size = probe.size();
        assert!(size.width.get().is_finite());
        assert!(size.height.get().is_finite());
        assert!(size.width.get() > 0.0);
    }

    #[test]
    fn inline_widget_widens_the_paragraph() {
        let probe = Probe::new();
        let fonts = Rc::new(Fonts::new());

        let span = TextSpan::new("icon ")
            .children([InlineSpan::Widget(SizedBox::new().width(40).height(20))]);

        let mut tester =
            WidgetTester::mount(probe.wrap(Provide::new(fonts).child(RichText::new(span))));
        tester.resize_with(BoxConstraints::loose(Size::new(300, 300)));
        tester.pump(Duration::ZERO);

        let size = probe.size();
        assert!(size.width.get().is_finite());
        assert!(size.width.get() > 40.0);
    }
}
