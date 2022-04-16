use std::{any::TypeId, marker::PhantomData, rc::Rc};

use crate::{
    engine::{widget::WidgetBuilder, Data, NotifyCallback},
    widget::WidgetId,
};

mod context;

pub use context::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId(WidgetId, TypeId);

impl CallbackId {
    pub fn get_widget_id(&self) -> WidgetId {
        self.0
    }

    pub fn get_type_id(&self) -> TypeId {
        self.1
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

pub trait CallbackFunc<W>
where
    W: WidgetBuilder,
{
    fn call(&self, ctx: &mut CallbackContext<W>, args: Rc<dyn Data>);
}

pub struct CallbackFn<W, A, F>
where
    W: WidgetBuilder,
    A: 'static,
    F: Fn(&mut CallbackContext<W>, &A),
{
    phantom: PhantomData<(W, A, F)>,

    func: F,
}

impl<W, A, F> CallbackFn<W, A, F>
where
    W: WidgetBuilder,
    A: 'static,
    F: Fn(&mut CallbackContext<W>, &A),
{
    pub fn new(func: F) -> Self {
        Self {
            phantom: PhantomData,

            func,
        }
    }
}

impl<W, A, F> CallbackFunc<W> for CallbackFn<W, A, F>
where
    W: WidgetBuilder,
    A: Data,
    F: Fn(&mut CallbackContext<W>, &A),
{
    fn call(&self, ctx: &mut CallbackContext<W>, args: Rc<dyn Data>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}
