use std::rc::Rc;

use fnv::FnvHashSet;

use crate::{
    callback::{Callback, CallbackId, CallbackQueue},
    manager::widgets::node::WidgetNode,
    unit::Data,
    util::tree::Tree,
    widget::WidgetId,
};

pub struct PluginContext<'ctx> {
    pub(crate) tree: &'ctx Tree<WidgetId, WidgetNode>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) callback_queue: CallbackQueue,
}

impl PluginContext<'_> {
    pub fn get_widgets(&self) -> &Tree<WidgetId, WidgetNode> {
        self.tree
    }

    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    pub fn call<A>(&mut self, callback: Callback<A>, args: A)
    where
        A: Data,
    {
        if let Some(callback_id) = callback.get_id() {
            self.callback_queue
                .lock()
                .push((callback_id, Rc::new(args)));
        }
    }

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub unsafe fn call_unsafe(&mut self, callback_id: CallbackId, args: Rc<dyn Data>) {
        self.callback_queue.lock().push((callback_id, args));
    }
}
