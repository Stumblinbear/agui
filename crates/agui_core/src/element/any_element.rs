use std::any::Any;

use crate::element::Element;

/// The type-erased, object-safe form of [`Element`].
pub trait AnyElement {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn element_name(&self) -> &str;
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
}

impl Element for Box<dyn AnyElement> {}
