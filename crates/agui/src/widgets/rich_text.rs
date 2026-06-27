use crate::{
    prelude::{element::*, render_object::*},
    widget::ChildrenElement,
};

/// A widget that lays out a tree of styled spans, with inline widgets flowing alongside the text.
///
/// It reads the ambient [`Fonts`] from scope, so wrap it (directly or above) in a
/// [`Provide`](crate::provide::Provide) of a `Fonts` for it to shape against.
pub struct RichText<Children = ()> {
    span: TextSpan<Children>,
}

impl<Children> RichText<Children> {
    pub fn new(span: impl Into<TextSpan<Children>>) -> Self {
        Self { span: span.into() }
    }
}

impl<Children> Widget for RichText<Children>
where
    Children: Widget + 'static,
    Children::Render: RenderBox + Sized + 'static,
    Children::Element: 'static,
{
    type Element = RichTextElement<Children>;

    type Render = RenderParagraph<dyn RenderBox>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let (content, widgets) = self.span.flatten();

        RichTextElement {
            inner: ChildrenElement::new(ctx, widgets, |renders| {
                let mut paragraph = RenderParagraph::new(content);
                paragraph.set_children(renders);
                paragraph
            }),
        }
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let (content, widgets) = self.span.flatten();

        let changed = element.inner.render_object_mut().set_content(content);

        element.inner.update(ctx, widgets);

        if changed {
            ctx.mark_needs_semantics_update();
        }
    }
}

/// The [`Element`] of a [`RichText`]: it holds the inline child elements and the paragraph render object,
/// resolves the ambient [`Fonts`] at mount, and re-applies them whenever the provided value changes, leaving
/// the children in place.
pub struct RichTextElement<Children>
where
    Children: Widget + 'static,
    Children::Render: RenderBox + Sized + 'static,
    Children::Element: 'static,
{
    inner: ChildrenElement<Vec<Children>, RenderParagraph<dyn RenderBox>>,
}

// SAFETY: delegates all child management to its inner `ChildrenElement` and resolves its render object from it.
unsafe impl<Children> Element for RichTextElement<Children>
where
    Children: Widget + 'static,
    Children::Render: RenderBox + Sized + 'static,
    Children::Element: 'static,
{
    type Render = RenderParagraph<dyn RenderBox>;

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.inner.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        let fonts = ctx.depend_on_provided::<Fonts>();
        self.inner.render_object_mut().set_fonts(fonts);

        Element::mount(&mut self.inner, ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        Element::unmount(&mut self.inner, ctx);
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        let fonts = ctx.depend_on_provided::<Fonts>();
        self.inner.render_object_mut().set_fonts(fonts);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        Element::describe(&self.inner, d)
    }
}
