use std::sync::Arc;

use fnv::FnvHashSet;
use parking_lot::Mutex;

use crate::{
    engine::node::WidgetNode,
    notifiable::{state::StateMap, ListenerId, NotifiableValue, Notify},
    plugin::PluginId,
    tree::Tree,
    widget::WidgetId,
};

pub struct PluginContext<'ui, 'ctx> {
    pub(crate) plugin_id: PluginId,

    pub(crate) tree: &'ctx Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,

    pub(crate) changed: Arc<Mutex<FnvHashSet<ListenerId>>>,
}

impl<'ui, 'ctx> PluginContext<'ui, 'ctx> {
    pub fn get_tree(&self) -> &'ctx Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }

    pub fn mark_dirty(&self, listener_id: ListenerId) {
        self.changed.lock().insert(listener_id);
    }
}

// Globals
impl<'ui, 'ctx> PluginContext<'ui, 'ctx> {
    /// Initialize a global value if it's not set already. This does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.global.get_or(func)
    }

    /// Fetch a global value if it exists. The caller will be updated when the value is changed.
    pub fn try_use_global<V>(&mut self) -> Option<Notify<V>>
    where
        V: NotifiableValue,
    {
        self.global.add_listener::<V>(self.plugin_id.into());

        self.global.get::<V>()
    }

    /// Fetch a global value, or initialize it with `func`. The caller will be updated when the value is changed.
    pub fn use_global<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.global.add_listener::<V>(self.plugin_id.into());

        self.global.get_or(func)
    }
}
