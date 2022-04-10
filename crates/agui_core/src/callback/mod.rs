use std::{any::TypeId, marker::PhantomData};

use crate::{
    engine::{notify::NotifyCallback, Data},
    widget::WidgetId,
};

mod context;

pub use context::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId(pub WidgetId, pub TypeId);

#[derive(Clone)]
pub struct Callback<A>
where
    A: Data,
{
    phantom: PhantomData<A>,

    callback_id: Option<CallbackId>,
    notifier_callbacks: Option<NotifyCallback>,
}

impl<A> Default for Callback<A>
where
    A: Data,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData,

            callback_id: None,
            notifier_callbacks: None,
        }
    }
}

impl<A> Callback<A>
where
    A: Data,
{
    pub(crate) fn new(callback_id: CallbackId) -> Self {
        Self {
            phantom: PhantomData,

            callback_id: Some(callback_id),
            notifier_callbacks: None,
        }
    }

    pub fn emit(&self, args: A) {
        if let Some(callback_id) = self.callback_id {
            if let Some(notifier) = &self.notifier_callbacks {
                notifier.lock().push((callback_id, Box::new(args)));
            }
        }
    }
}

pub trait CallbackFunc<S>
where
    S: Data,
{
    fn call(&self, ctx: &mut CallbackContext<S>, args: Box<dyn Data>);
}

pub struct CallbackFn<F, S, A>
where
    F: Fn(&mut CallbackContext<S>, &A),
    S: Data,
    A: 'static,
{
    phantom: PhantomData<(F, S, A)>,

    func: F,
}

impl<F, S, A> CallbackFn<F, S, A>
where
    F: Fn(&mut CallbackContext<S>, &A),
    S: Data,
    A: 'static,
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<F, S, A> CallbackFunc<S> for CallbackFn<F, S, A>
where
    F: Fn(&mut CallbackContext<S>, &A),
    S: Data,
    A: Data,
{
    fn call(&self, ctx: &mut CallbackContext<S>, args: Box<dyn Data>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}
