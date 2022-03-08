use std::{any::TypeId, marker::PhantomData, rc::Rc, sync::Arc};

use crate::{
    engine::notify::{Notifier, NotifyCallback},
    state::StateValue,
};

use super::{CallbackContext, WidgetId};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId(pub(crate) WidgetId, pub(crate) TypeId);

#[derive(Clone)]
pub struct Callback<A>
where
    A: StateValue + Clone,
{
    phantom: PhantomData<A>,

    callback_id: Option<CallbackId>,
    notifier_callbacks: Option<NotifyCallback>,
}

impl<A> Default for Callback<A>
where
    A: StateValue + Clone,
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
    A: StateValue + Clone,
{
    pub(crate) fn new(callback_id: CallbackId, notifier: Rc<Notifier>) -> Self {
        Self {
            phantom: PhantomData,

            callback_id: Some(callback_id),
            notifier_callbacks: Some(Arc::clone(&notifier.callbacks)),
        }
    }

    pub fn emit(&self, args: A) {
        if let Some(callback_id) = self.callback_id {
            self.notifier_callbacks
                .as_ref()
                .unwrap()
                .lock()
                .push((callback_id, Box::new(args)));
        }
    }
}

pub trait CallbackFunc<'ui> {
    fn call(&self, ctx: &mut CallbackContext<'ui, '_>, args: Box<dyn StateValue>);
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
    A: StateValue + Clone,
{
    fn call(&self, ctx: &mut CallbackContext<'ui, '_>, args: Box<dyn StateValue>) {
        let args = args
            .downcast_ref::<A>()
            .expect("failed to downcast callback args");

        (self.func)(ctx, args)
    }
}
