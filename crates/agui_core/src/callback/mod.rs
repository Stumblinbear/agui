use std::{any::TypeId, marker::PhantomData, rc::Rc};

use crate::{
    manager::{CallbackQueue, Data},
    widget::{WidgetBuilder, WidgetId},
};

mod context;
mod func;

pub use context::*;
pub use func::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CallbackId {
    widget_id: WidgetId,
    type_id: TypeId,
}

impl CallbackId {
    pub fn get_widget_id(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_type_id(&self) -> TypeId {
        self.type_id
    }
}

#[derive(Debug, Default, Clone)]
pub struct Callback<A>
where
    A: Data,
{
    phantom: PhantomData<A>,

    id: Option<CallbackId>,

    callback_queue: Option<CallbackQueue>,
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<A> Send for Callback<A> where A: Data {}
unsafe impl<A> Sync for Callback<A> where A: Data {}

impl<A> Callback<A>
where
    A: Data,
{
    pub(crate) fn new<F, W>(widget_id: WidgetId, callback_queue: CallbackQueue) -> Self
    where
        W: WidgetBuilder,
        F: Fn(&mut CallbackContext<W>, &A) + 'static,
    {
        Self {
            phantom: PhantomData,

            id: Some(CallbackId {
                widget_id,
                type_id: TypeId::of::<F>(),
            }),

            callback_queue: Some(callback_queue),
        }
    }

    pub fn get_id(&self) -> Option<CallbackId> {
        self.id
    }

    pub fn is_some(&self) -> bool {
        self.id.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.id.is_none()
    }

    pub fn call(&self, args: A) {
        unsafe {
            self.call_unsafe(Rc::new(args));
        }
    }

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub unsafe fn call_unsafe(&self, args: Rc<dyn Data>) {
        if let Some(callback_queue) = &self.callback_queue {
            if let Some(callback_id) = self.id {
                callback_queue.lock().push((callback_id, args));
            }
        }
    }
}
