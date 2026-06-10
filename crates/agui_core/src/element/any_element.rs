use std::any::Any;

use crate::{
    context::Dispatch,
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, RoutingId},
    render_object::AnyRenderObject,
};

/// The type-erased, object-safe form of [`Element`].
pub trait AnyElement {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn element_name(&self) -> &str;

    fn dyn_dispatch(
        &mut self,
        render: &mut dyn AnyRenderObject,
        path: &[RoutingId],
        action: Dispatch,
    );

    fn dyn_describe(&self, d: &mut Diagnostics) -> DiagnosticsNode;
}

impl<T> AnyElement for T
where
    T: Any + Element,
    T::Render: AnyRenderObject + Sized,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn element_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_dispatch(
        &mut self,
        render: &mut dyn AnyRenderObject,
        path: &[RoutingId],
        action: Dispatch,
    ) {
        let render = render
            .as_any_mut()
            .downcast_mut::<T::Render>()
            .expect("render type does not match the element it is threaded to");

        self.dispatch(render, path, action);
    }

    fn dyn_describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.describe(d)
    }
}

impl Element for Box<dyn AnyElement> {
    type Render = dyn AnyRenderObject;

    fn dispatch(
        &mut self,
        render: &mut (dyn AnyRenderObject + 'static),
        path: &[RoutingId],
        action: Dispatch,
    ) {
        (**self).dyn_dispatch(render, path, action);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        (**self).dyn_describe(d)
    }
}
