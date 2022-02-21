use std::any::TypeId;

use crate::{
    engine::node::WidgetNode,
    state::{map::StateMap, ListenerId, State, StateValue},
    tree::Tree,
    unit::{LayoutType, Rect, Ref, Size},
    widget::WidgetId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HandlerType {
    Effect,
    Computed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HandlerId(HandlerType, TypeId);

impl HandlerId {
    pub fn of<F>(ty: HandlerType) -> Self
    where
        F: ?Sized + 'static,
    {
        Self(ty, TypeId::of::<F>())
    }

    pub fn get_type(&self) -> HandlerType {
        self.0
    }
}

pub struct WidgetContext<'ui, 'ctx> {
    pub(crate) widget_id: WidgetId,
    pub(crate) handler_id: HandlerId,

    pub(crate) tree: &'ctx mut Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,

    pub(crate) widget: &'ctx mut WidgetNode<'ui>,
}

impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    pub fn get_widget(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_listener(&self) -> ListenerId {
        (self.widget_id, self.handler_id).into()
    }

    pub fn get_tree(&mut self) -> &Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }
}

// Globals
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
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

// Local state
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    /// Initializing a state does not cause the initializer to be updated when its value is changed.
    pub fn init_state<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue,
        F: FnOnce() -> V,
    {
        self.widget.state.get_or::<V, F>(None, func)
    }

    /// Fetch a local state value, or initialize it with `func` if it doesn't exist. The caller will be updated when the value is changed.
    pub fn use_state<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue,
        F: FnOnce() -> V,
    {
        self.widget
            .state
            .get_or::<V, F>(Some(self.get_listener()), func)
    }

    pub fn use_state_from<V, F>(&mut self, widget_id: WidgetId, func: F) -> State<V>
    where
        V: StateValue,
        F: FnOnce() -> V,
    {
        let listener_id = self.get_listener();

        let target_widget = self
            .tree
            .get_mut(widget_id)
            .expect("cannot use state from a widget that doesn't exist");

        target_widget.state.get_or::<V, F>(Some(listener_id), func)
    }
}

// Layout
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    /// Fetch the layout of a widget.
    pub fn get_layout_type(&self) -> Ref<LayoutType> {
        Ref::clone(&self.widget.layout_type)
    }

    pub fn get_rect(&self) -> Option<Rect> {
        self.widget.rect
    }

    pub fn get_size(&self) -> Option<Size> {
        self.widget.rect.map(|rect| rect.into())
    }
}
