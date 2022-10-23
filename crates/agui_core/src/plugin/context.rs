use fnv::FnvHashSet;

use crate::{
    callback::{Callback, CallbackId, CallbackQueue},
    manager::element::WidgetElement,
    unit::Data,
    util::tree::Tree,
    widget::WidgetId,
};

pub struct PluginContext<'ctx> {
    pub(crate) tree: &'ctx Tree<WidgetId, WidgetElement>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) callback_queue: CallbackQueue,
}

impl PluginContext<'_> {
    pub fn get_widgets(&self) -> &Tree<WidgetId, WidgetElement> {
        self.tree
    }

    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    pub fn call<A>(&mut self, callback: &Callback<A>, arg: A)
    where
        A: Data,
    {
        self.callback_queue.call(callback, arg);
    }

    /// # Panics
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub fn call_unchecked(&mut self, callback_id: CallbackId, arg: Box<dyn Data>) {
        self.callback_queue.call_unchecked(callback_id, arg);
    }

    pub fn call_many<A>(&mut self, callbacks: &[Callback<A>], arg: A)
    where
        A: Data,
    {
        self.callback_queue.call_many(callbacks, arg);
    }

    /// # Panics
    ///
    /// You must ensure the callbacks are expecting the type of the `arg` passed in. If the type
    /// is different, it will panic.
    pub fn call_many_unchecked(&mut self, callback_ids: &[CallbackId], arg: Box<dyn Data>) {
        self.callback_queue.call_many_unchecked(callback_ids, arg);
    }
}
