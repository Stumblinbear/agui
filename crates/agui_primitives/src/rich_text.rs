use std::rc::Rc;

use agui_render::prelude::{element::*, render_object::*};

/// A widget that lays out a tree of styled spans, with inline widgets flowing alongside the text.
pub struct RichText<Children = ()> {
    span: TextSpan<Children>,
}

impl<Children> RichText<Children> {
    pub fn new(span: impl Into<TextSpan<Children>>) -> Self {
        Self { span: span.into() }
    }
}

/// The element of a [`RichText`], holding its inline child elements in span order.
pub struct RichTextElement<C> {
    fonts: Option<Rc<Fonts>>,
    children: Vec<ElementNode<C>>,
}

impl<C> Element for RichTextElement<C>
where
    C: Element,
    C::Render: Sized,
{
    type Render = RenderParagraph<C::Render>;

    fn dispatch(
        &mut self,
        render: &mut RenderParagraph<C::Render>,
        path: &RoutingPath,
        action: Dispatch,
    ) {
        let Some((head, rest)) = path.decode() else {
            return;
        };

        let index = head.get() as usize;
        let renders = render.children_mut();

        if index < self.children.len() && index < renders.len() {
            self.children[index]
                .element
                .dispatch(&mut renders[index].object, rest, action);
        }
    }
}

impl<Children> Widget for RichText<Children>
where
    Children: Widget + 'static,
    Children::Render: RenderBox,
{
    type Element = RichTextElement<Children::Element>;

    type Render = RenderParagraph<Children::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (content, widgets) = self.span.flatten();

        let mut render_object = RenderParagraph::new(content);

        let fonts = ctx.depend_on_provided::<Fonts>();
        render_object.set_fonts(fonts.clone());

        let mut children = Vec::with_capacity(widgets.len());
        let mut render_children = Vec::with_capacity(widgets.len());

        for (index, widget) in widgets.into_iter().enumerate() {
            let (element, child_render) = ctx
                .with_routing_id(RoutingId::new(u32::try_from(index).unwrap()), |ctx| {
                    widget.create(ctx)
                });
            children.push(ElementNode::new(element));
            render_children.push(RenderNode::new(child_render));
        }

        render_object.set_children(render_children);

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

        element.fonts = ctx.depend_on_provided::<Fonts>();
        render_object.set_fonts(element.fonts.clone());

        let mut old_children = std::mem::take(&mut element.children).into_iter();
        let mut old_render = render_object.take_children().into_iter();

        let mut new_children = Vec::with_capacity(widgets.len());
        let mut new_render = Vec::with_capacity(widgets.len());

        for (index, widget) in widgets.into_iter().enumerate() {
            ctx.with_routing_id(RoutingId::new(u32::try_from(index).unwrap()), |ctx| match (
                old_children.next(),
                old_render.next(),
            ) {
                (Some(mut child), Some(mut render)) => {
                    widget.update(&mut child.element, &mut render.object, ctx);
                    new_children.push(child);
                    new_render.push(render);
                }
                _ => {
                    let (child, mut render) = widget.create(ctx);
                    ctx.mount(&mut render);
                    new_children.push(ElementNode::new(child));
                    new_render.push(RenderNode::new(render));
                }
            });
        }

        element.children = new_children;
        render_object.set_children(new_render);
    }
}

#[cfg(test)]
mod harness {
    use std::time::Duration;

    use agui_render::prelude::{
        element::Size,
        render_object::{BoxConstraints, Fonts, InlineSpan, TextSpan, TextStyle},
    };

    use agui_render::provide::Provide;
    use agui_test::{Probe, WidgetTester};

    use super::RichText;
    use crate::sized_box::SizedBox;

    #[test]
    fn mixed_styles_produce_a_finite_size() {
        let probe = Probe::new();
        let fonts = Fonts::new();

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
        let fonts = Fonts::new();

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
