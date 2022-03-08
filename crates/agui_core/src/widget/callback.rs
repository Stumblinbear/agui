use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use super::{CallbackContext, WidgetId};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId(pub(crate) WidgetId, pub(crate) TypeId);

#[derive(Copy, Clone)]
pub struct Callback<A>(pub(crate) PhantomData<A>, pub(crate) Option<CallbackId>);

impl<A> Default for Callback<A> {
    fn default() -> Self {
        Self(PhantomData, None)
    }
}

impl<A> Callback<A> {
    pub fn get_id(&self) -> Option<CallbackId> {
        self.1
    }
}

pub trait CallbackFunc<'ui> {
    fn call(&self, ctx: &mut CallbackContext<'ui, '_>, args: Box<dyn Any>);
}

pub struct CallbackFn<'ui, F, A>
where
    F: Fn(&mut CallbackContext<'ui, '_>, &A),
    A: 'static,
{
    phantom: PhantomData<(&'ui F, A)>,

    func: F,
}

impl<'ui, F, A> CallbackFn<'ui, F, A>
where
    F: Fn(&mut CallbackContext<'ui, '_>, &A),
    A: 'static,
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<'ui, F, A> CallbackFunc<'ui> for CallbackFn<'ui, F, A>
where
    F: Fn(&mut CallbackContext<'ui, '_>, &A),
    A: 'static,
{
    fn call(&self, ctx: &mut CallbackContext<'ui, '_>, args: Box<dyn Any>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}
