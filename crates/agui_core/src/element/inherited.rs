use std::any::{Any, TypeId};

use crate::widget::Widget;

use super::lifecycle::ElementLifecycle;

pub trait ElementInherited: ElementLifecycle {
    type Data: Any + ?Sized
    where
        Self: Sized;

    fn inherited_data(&self) -> &Self::Data
    where
        Self: Sized;

    fn child(&self) -> Widget;

    /// Called during the build phase to determine if elements listening to
    /// this element need to be rebuilt.
    fn needs_notify(&mut self) -> bool;
}

pub trait ErasedElementInherited: ElementLifecycle {
    /// Returns the [`TypeId`] of the data provided by this element.
    fn inherited_type_id(&self) -> TypeId;

    fn child(&self) -> Widget;

    /// Called during the build phase to determine if elements listening to
    /// this element need to be rebuilt.
    fn needs_notify(&mut self) -> bool;
}

impl<T> ErasedElementInherited for T
where
    T: ElementInherited,
    T::Data: Any,
{
    fn inherited_type_id(&self) -> TypeId {
        TypeId::of::<T::Data>()
    }

    fn child(&self) -> Widget {
        self.child()
    }

    fn needs_notify(&mut self) -> bool {
        self.needs_notify()
    }
}
