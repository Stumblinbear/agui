use std::{any::TypeId, marker::PhantomData, rc::Rc};

use crate::{
    engine::{Data, NotifyCallback},
    widget::WidgetId,
};

mod context;

pub use context::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId(WidgetId, TypeId);

impl CallbackId {
    pub fn get_widget_id(&self) -> WidgetId {
        self.0
    }
}

#[derive(Clone)]
pub struct Callback<A>
where
    A: Data,
{
    phantom: PhantomData<A>,

    callback_id: Option<CallbackId>,
    notifier: Option<NotifyCallback>,
}

impl<A> Default for Callback<A>
where
    A: Data,
{
    fn default() -> Self {
        Self {
            phantom: PhantomData,

            callback_id: None,
            notifier: None,
        }
    }
}

impl<A> std::fmt::Debug for Callback<A>
where
    A: Data,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.callback_id.fmt(f)
    }
}

impl<A> Callback<A>
where
    A: Data,
{
    pub(crate) fn new<F, S>(notifier: NotifyCallback, widget_id: WidgetId) -> Self
    where
        S: Data,
        F: Fn(&mut CallbackContext<S>, &A) + 'static,
    {
        Self {
            phantom: PhantomData,

            callback_id: Some(CallbackId(widget_id, TypeId::of::<F>())),
            notifier: Some(notifier),
        }
    }

    pub fn get_id(&self) -> Option<CallbackId> {
        self.callback_id
    }

    pub fn is_some(&self) -> bool {
        matches!(self.callback_id, Some(_))
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub fn emit(&self, args: A) {
        if let Some(callback_id) = self.callback_id {
            if let Some(notifier) = &self.notifier {
                notifier.lock().push((callback_id, Rc::new(args)));
            }
        }
    }
}

pub trait CallbackFunc<S>
where
    S: Data,
{
    fn call(&self, ctx: &mut CallbackContext<S>, args: Rc<dyn Data>);
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
    fn call(&self, ctx: &mut CallbackContext<S>, args: Rc<dyn Data>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}
