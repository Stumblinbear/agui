use std::{any::TypeId, marker::PhantomData, rc::Rc};

use crate::{
    engine::{widget::WidgetBuilder, Data},
    widget::WidgetId,
};

mod context;

pub use context::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Callback<A>
where
    A: Data,
{
    None,
    Some {
        phantom: PhantomData<A>,

        id: CallbackId,
    },
}

impl<A> Copy for Callback<A> where A: Data + Clone {}

impl<A> Default for Callback<A>
where
    A: Data,
{
    fn default() -> Self {
        Self::None
    }
}

impl<A> Callback<A>
where
    A: Data,
{
    pub(crate) fn new<F, S>(widget_id: WidgetId) -> Self
    where
        S: Data,
        F: Fn(&mut CallbackContext<S>, &A) + 'static,
    {
        Self::Some {
            phantom: PhantomData,

            id: CallbackId {
                widget_id,
                type_id: TypeId::of::<F>(),
            },
        }
    }

    pub fn get_id(&self) -> Option<CallbackId> {
        match self {
            Self::None => None,
            Self::Some { id, .. } => Some(*id),
        }
    }

    pub fn is_some(&self) -> bool {
        matches!(self, Callback::Some { .. })
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
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
