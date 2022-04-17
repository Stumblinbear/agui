use std::{any::TypeId, marker::PhantomData, sync::Arc};

use crate::{
    engine::{widget::WidgetBuilder, ArcEmitCallbacks, Data},
    widget::WidgetId,
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
    pub(crate) fn new<F, W>(widget_id: WidgetId) -> Self
    where
        W: WidgetBuilder,
        F: Fn(&mut CallbackContext<W>, &A) + 'static,
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

    pub(crate) fn as_arc(&self, arc_emit_callbacks: ArcEmitCallbacks) -> ArcCallback<A> {
        ArcCallback {
            phantom: PhantomData,

            id: self.get_id().unwrap(),
            arc_emit_callbacks,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArcCallback<A>
where
    A: Data,
{
    pub phantom: PhantomData<A>,

    pub id: CallbackId,

    pub(crate) arc_emit_callbacks: ArcEmitCallbacks,
}

impl<A> ArcCallback<A>
where
    A: Data,
{
    pub fn emit(&self, args: A) {
        self.arc_emit_callbacks
            .lock()
            .push((self.id, Arc::new(args)));
    }

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub unsafe fn emit_unsafe(&self, args: Arc<dyn Data>) {
        self.arc_emit_callbacks.lock().push((self.id, args));
    }
}

#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl<A> Send for ArcCallback<A> where A: Data {}
unsafe impl<A> Sync for ArcCallback<A> where A: Data {}

impl<A> From<ArcCallback<A>> for Callback<A>
where
    A: Data,
{
    fn from(callback: ArcCallback<A>) -> Self {
        Self::Some {
            phantom: PhantomData,

            id: callback.id,
        }
    }
}
