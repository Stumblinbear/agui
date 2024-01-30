use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::Arc,
};

use crate::{callback::strategies::CallbackStrategy, element::ElementId};

pub mod strategies;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId {
    element_id: ElementId,
    type_id: TypeId,
}

impl CallbackId {
    pub fn element_id(&self) -> ElementId {
        self.element_id
    }
}

pub struct Callback<A: ?Sized> {
    strategy: Arc<dyn CallbackStrategy>,

    phantom: PhantomData<A>,
    callback_id: CallbackId,
}

impl<A> Callback<A>
where
    A: Any + ?Sized,
{
    pub fn new<F>(strategy: Arc<dyn CallbackStrategy>, element_id: ElementId) -> Self
    where
        F: 'static,
    {
        Self {
            strategy,
            phantom: PhantomData,
            callback_id: CallbackId {
                element_id,
                type_id: TypeId::of::<F>(),
            },
        }
    }

    pub fn id(&self) -> CallbackId {
        self.callback_id
    }
}

impl<A> Callback<A>
where
    A: Any + Send,
{
    pub fn call(&self, arg: A) {
        self.strategy
            .call_unchecked(self.callback_id, Box::new(arg))
    }

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub fn call_unchecked(&self, arg: Box<dyn Any + Send>) {
        self.strategy.call_unchecked(self.callback_id, arg)
    }
}

impl<A> PartialEq for Callback<A>
where
    A: ?Sized,
{
    fn eq(&self, other: &Self) -> bool {
        self.callback_id == other.callback_id
    }
}

impl<A> Clone for Callback<A>
where
    A: ?Sized,
{
    fn clone(&self) -> Self {
        Self {
            strategy: Arc::clone(&self.strategy),

            phantom: PhantomData,
            callback_id: self.callback_id,
        }
    }
}

impl<A> std::fmt::Debug for Callback<A>
where
    A: Any,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Callback")
            .field("callback_id", &self.callback_id)
            .finish()
    }
}

unsafe impl<A: ?Sized> Send for Callback<A> {}
unsafe impl<A: ?Sized> Sync for Callback<A> {}
