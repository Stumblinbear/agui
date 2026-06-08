use std::{any::Any, rc::Rc};

use agui_core::{
    context::{Dispatch, UpdateCtx},
    element::{Element, RoutingId, node::ElementNode},
    widget::Widget,
};

/// Makes `value` available to the subtree below it, captured by descendants that ask for the type.
///
/// The value is shared, so descendants hold it past the build; the widget itself draws nothing and
/// passes its child through unchanged.
pub struct Provide<T, Child> {
    value: Rc<T>,
    child: Child,
}

impl<T> Provide<T, ()> {
    pub fn new(value: Rc<T>) -> Self {
        Self { value, child: () }
    }
}

impl<T> Provide<T, ()> {
    pub fn child<Child>(self, child: Child) -> Provide<T, Child> {
        Provide {
            value: self.value,
            child,
        }
    }
}

impl<T, Child> Widget for Provide<T, Child>
where
    T: Any,
    Child: Widget,
{
    type Element = ProvideElement<T, Child::Element>;

    type Render = Child::Render;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, render_object) =
            ctx.with_provided(Rc::clone(&self.value), |ctx| self.child.create(ctx));

        (
            ProvideElement {
                child: ElementNode::new(element),
                value: self.value,
            },
            render_object,
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        element.value = Rc::clone(&self.value);

        ctx.with_provided(self.value, |ctx| {
            self.child
                .update(&mut element.child.element, render_object, ctx);
        });
    }
}

/// The [`Element`] of a [`Provide`], re-applying the provided value when a rebuild reaches its subtree.
pub struct ProvideElement<T, C> {
    child: ElementNode<C>,
    value: Rc<T>,
}

impl<T, C> Element for ProvideElement<T, C>
where
    T: Any,
    C: Element,
    C::Render: Sized,
{
    type Render = C::Render;

    fn dispatch(&mut self, render: &mut C::Render, path: &[RoutingId], action: Dispatch) {
        match action {
            Dispatch::Rebuild(ctx) => ctx.with_provided(Rc::clone(&self.value), |ctx| {
                self.child
                    .element
                    .dispatch(render, path, Dispatch::Rebuild(ctx));
            }),

            action @ Dispatch::Message(_) => self.child.element.dispatch(render, path, action),
        }
    }
}
