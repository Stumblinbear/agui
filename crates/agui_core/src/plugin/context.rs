use std::rc::Rc;

use fnv::FnvHashSet;

use crate::{
    callback::CallbackId,
    engine::{tree::Tree, Data, NotifyCallback},
    widget::{Widget, WidgetId},
};

pub struct PluginContext<'ctx> {
    pub(crate) tree: &'ctx Tree<WidgetId, Widget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
    pub(crate) notifier: NotifyCallback,
}

impl PluginContext<'_> {
    pub fn get_tree(&self) -> &Tree<WidgetId, Widget> {
        self.tree
    }

    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    pub fn notify<A>(&mut self, callback_id: CallbackId, args: A)
    where
        A: Data,
    {
        self.notifier.lock().push((callback_id, Rc::new(args)));
    }

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub unsafe fn notify_unsafe(&mut self, callback_id: CallbackId, args: Rc<dyn Data>) {
        self.notifier.lock().push((callback_id, args));
    }
}
