use crate::{
    context::{Dispatch, UpdateCtx},
    element::Element,
    routing_id::RoutingId,
    widget::Widget,
};

/// Owns a single [`Element`](crate::element::Element) in the element tree.
pub struct ElementNode<E> {
    pub element: E,
}

impl<E> ElementNode<E> {
    pub fn new(element: E) -> Self {
        Self { element }
    }
}

impl<E> ElementNode<E>
where
    E: Element,
{
    #[inline]
    pub fn update<W>(&mut self, new: &W, old: &W, ctx: &mut UpdateCtx)
    where
        W: Widget<Element = E>,
    {
        new.update(&mut self.element, old, ctx);
    }

    #[inline]
    pub fn dispatch<W>(&mut self, new: &W, path: &[RoutingId], action: Dispatch)
    where
        W: Widget<Element = E>,
    {
        new.dispatch(&mut self.element, path, action);
    }

    #[inline]
    pub fn create_render_object<W>(&mut self, new: &W) -> W::Render
    where
        W: Widget<Element = E>,
    {
        new.create_render_object(&self.element)
    }

    #[inline]
    pub fn update_render_object<W>(&mut self, new: &W, render_object: &mut W::Render)
    where
        W: Widget<Element = E>,
    {
        new.update_render_object(&self.element, render_object);
    }
}
