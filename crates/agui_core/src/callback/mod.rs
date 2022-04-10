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

pub struct CallbackFn<S, A, F>
where
    S: Data,
    A: 'static,
    F: Fn(&mut CallbackContext<S>, &A),
{
    phantom: PhantomData<(S, A, F)>,

    func: F,
}

impl<S, A, F> CallbackFn<S, A, F>
where
    S: Data,
    A: 'static,
    F: Fn(&mut CallbackContext<S>, &A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<S, A, F> CallbackFunc<S> for CallbackFn<S, A, F>
where
    S: Data,
    A: Data,
    F: Fn(&mut CallbackContext<S>, &A),
{
    fn call(&self, ctx: &mut CallbackContext<S>, args: Box<dyn Data>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}
