use crate::{
    engine::{node::WidgetNode, ChangedListeners},
    plugin::PluginId,
    state::{map::StateMap, ListenerId, State, StateValue},
    tree::Tree,
    widget::WidgetId,
};

pub struct PluginContext<'ui, 'ctx> {
    pub(crate) plugin_id: PluginId,

    pub(crate) tree: &'ctx Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,

    pub(crate) changed_listeners: ChangedListeners,
}

impl<'ui, 'ctx> PluginContext<'ui, 'ctx> {
    pub fn get_tree(&self) -> &'ctx Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }

    pub fn get_listener(&self) -> ListenerId {
        self.plugin_id.into()
    }

    pub fn mark_dirty(&mut self, listener_id: ListenerId) {
        self.changed_listeners.notify(listener_id);
    }
}

// Globals
impl<'ui, 'ctx> PluginContext<'ui, 'ctx> {
    /// Fetch a global value if it exists. The caller will be updated when the value is changed.
    pub fn try_use_global<V>(&mut self) -> Option<State<V>>
    where
        V: StateValue,
    {
        self.global.try_get::<V>(Some(self.get_listener()))
    }

    /// Initialize a global value if it's not set already. This does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue,
        F: FnOnce() -> V,
    {
        self.global.get_or(None, func)
    }

    /// Fetch a global value, or initialize it with `func`. The caller will be updated when the value is changed.
    pub fn use_global<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue,
        F: FnOnce() -> V,
    {
        self.global.get_or(Some(self.get_listener()), func)
    }
}
