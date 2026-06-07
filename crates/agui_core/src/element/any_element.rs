use std::any::Any;

use crate::{
    context::Dispatch,
    element::{Element, RoutingId},
};

/// The type-erased, object-safe form of [`Element`].
pub trait AnyElement {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn element_name(&self) -> &str;

    fn dyn_dispatch(&mut self, path: &[RoutingId], action: Dispatch);
}

impl<T> AnyElement for T
where
    T: Any + Element,
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

    fn dyn_dispatch(&mut self, path: &[RoutingId], action: Dispatch) {
        self.dispatch(path, action);
    }
}

impl Element for Box<dyn AnyElement> {
    fn dispatch(&mut self, path: &[RoutingId], action: Dispatch) {
        (**self).dyn_dispatch(path, action);
    }
}
