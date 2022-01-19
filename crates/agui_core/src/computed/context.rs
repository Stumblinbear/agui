use crate::{
    computed::ComputedId,
    engine::node::WidgetNode,
    notifiable::{state::StateMap, ListenerId, NotifiableValue, Notify},
    tree::Tree,
    unit::{LayoutType, Rect, Ref},
    widget::WidgetId,
};

pub struct ComputedContext<'ui, 'ctx> {
    pub(crate) widget_id: WidgetId,
    pub(crate) computed_id: ComputedId,

    pub(crate) widget: &'ctx mut WidgetNode<'ui>,

    pub(crate) tree: &'ctx Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,
}

impl<'ui, 'ctx> ComputedContext<'ui, 'ctx> {
    pub fn get_widget(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_tree(&self) -> &'ctx Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }
}

// Globals
impl<'ui, 'ctx> ComputedContext<'ui, 'ctx> {
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
        self.global
            .add_listener::<V>((self.widget_id, self.computed_id).into());

        self.global.get::<V>()
    }

    /// Fetch a global value, or initialize it with `func`. The caller will be updated when the value is changed.
    pub fn use_global<V, F>(&mut self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.global
            .add_listener::<V>((self.widget_id, self.computed_id).into());

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
        self.widget
            .state
            .add_listener::<V>((self.widget_id, self.computed_id).into());

        self.widget.state.get_or::<V, F>(func)
    }

    pub fn use_state_of<V, F>(&mut self, listener_id: ListenerId, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.widget.state.add_listener::<V>(listener_id);

        self.widget.state.get_or::<V, F>(func)
    }
}

// Layout
impl<'ui, 'ctx> ComputedContext<'ui, 'ctx> {
    /// Fetch the layout of a widget.
    pub fn get_layout_type(&self) -> Ref<LayoutType> {
        Ref::clone(&self.widget.layout_type)
    }

    /// Listen to the visual rect of the widget.
    pub fn use_rect(&mut self) -> Option<Rect> {
        self.widget
            .rect
            .add_listener((self.widget_id, self.computed_id).into());

        if self.widget.rect.has_value() {
            Some(*self.widget.rect.read())
        } else {
            None
        }
    }
}
