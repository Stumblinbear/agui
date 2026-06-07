use std::{any::Any, rc::Rc};

use agui_core::prelude::element::*;

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
    type Element = SingleChildElement<Child::Element>;

    type Render = Child::Render;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        ctx.with_provided(Rc::clone(&self.value), |ctx| {
            SingleChildElement::new(&self.child, ctx)
        })
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        ctx.with_provided(Rc::clone(&self.value), |ctx| {
            element.update(&self.child, &old.child, ctx);
        });
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(&self.child, path, action);
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        element.create_render_object(&self.child)
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        element.update_render_object(&self.child, render_object);
    }
}
