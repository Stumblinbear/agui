use crate::{
    computed::ComputedId,
    engine::node::WidgetNode,
    notifiable::{state::StateMap, ListenerId, NotifiableValue, Notify},
    tree::Tree,
    unit::{LayoutType, Ref},
    widget::WidgetId,
};

pub struct ComputedContext<'ui, 'ctx> {
    pub(crate) widget_id: WidgetId,
    pub(crate) computed_id: ComputedId,

    pub(crate) widget: &'ctx mut WidgetNode<'ui>,

    pub(crate) tree: &'ctx Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx StateMap,
}

impl<'ui, 'ctx> ComputedContext<'ui, 'ctx> {
    pub fn get_widget(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_listener(&self) -> ListenerId {
        (self.widget_id, self.computed_id).into()
    }

    pub fn get_tree(&mut self) -> &Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }
}

// Globals
impl<'ui, 'ctx> ComputedContext<'ui, 'ctx> {
    /// Fetch a global value if it exists. The caller will be updated when the value is changed.
    pub fn try_use_global<V>(&mut self) -> Option<Notify<V>>
    where
        V: NotifiableValue,
    {
        self.global.add_listener::<V>(self.get_listener());

        self.global.try_get::<V>()
    }

    /// Initialize a global value if it's not set already. This does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.global.get_or(func)
    }

    /// Fetch a global value, or initialize it with `func`. The caller will be updated when the value is changed.
    pub fn use_global<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.global.add_listener::<V>(self.get_listener());

        self.global.get_or(func)
    }
}

// Local state
impl<'ui, 'ctx> ComputedContext<'ui, 'ctx> {
    /// Initializing a state does not cause the initializer to be updated when its value is changed.
    pub fn init_state<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.widget.state.get_or::<V, F>(func)
    }

    /// Fetch a local state value, or initialize it with `func` if it doesn't exist. The caller will be updated when the value is changed.
    pub fn use_state<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.widget.state.add_listener::<V>(self.get_listener());

        self.widget.state.get_or::<V, F>(func)
    }

    pub fn use_state_from<V, F>(&mut self, widget_id: WidgetId, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        let target_widget = self
            .tree
            .get(widget_id)
            .expect("cannot use state from a widget that doesn't exist");

        target_widget.state.add_listener::<V>(self.get_listener());

        target_widget.state.get_or::<V, F>(func)
    }
}

// Layout
impl<'ui, 'ctx> ComputedContext<'ui, 'ctx> {
    /// Fetch the layout of a widget.
    pub fn get_layout_type(&self) -> Ref<LayoutType> {
        Ref::clone(&self.widget.layout_type)
    }
}
