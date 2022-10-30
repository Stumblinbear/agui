use crate::{
    callback::{Callback, CallbackId},
    element::ElementId,
    unit::Data,
};

pub trait ContextMut {
    fn mark_dirty(&mut self, element_id: ElementId);

    fn call<A>(&mut self, callback: &Callback<A>, arg: A)
    where
        A: Data;

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `arg` passed in. If the type
    /// is different, it will panic.
    fn call_unchecked(&mut self, callback_id: CallbackId, arg: Box<dyn Data>);

    fn call_many<A>(&mut self, callbacks: &[Callback<A>], arg: A)
    where
        A: Data;

    /// # Panics
    ///
    /// You must ensure the callbacks are expecting the type of the `arg` passed in. If the type
    /// is different, it will panic.
    fn call_many_unchecked(&mut self, callback_ids: &[CallbackId], arg: Box<dyn Data>);
}
