use std::rc::Rc;

use fnv::FnvHashSet;

use crate::{
    callback::CallbackId,
    engine::{tree::Tree, ArcEmitCallbacks, Data, EmitCallbacks},
    widget::{Widget, WidgetId},
};

pub struct PluginContext<'ctx> {
    pub(crate) tree: &'ctx Tree<WidgetId, Widget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,

    pub(crate) emit_callbacks: &'ctx mut EmitCallbacks,
    pub(crate) arc_emit_callbacks: ArcEmitCallbacks,
}

impl PluginContext<'_> {
    pub fn get_tree(&self) -> &Tree<WidgetId, Widget> {
        self.tree
    }

    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }

    pub fn emit<A>(&mut self, callback_id: CallbackId, args: A)
    where
        A: Data,
    {
        self.emit_callbacks.push((callback_id, Rc::new(args)));
    }

    /// # Safety
    ///
    /// You must ensure the callback is expecting the type of the `args` passed in. If the type
    /// is different, it will panic.
    pub unsafe fn emit_unsafe(&mut self, callback_id: CallbackId, args: Rc<dyn Data>) {
        self.emit_callbacks.push((callback_id, args));
    }
}
