use std::rc::Rc;

use crate::{
    engine::{node::WidgetNode, notify::Notifier},
    state::{map::StateMap, ListenerId, State, StateValue},
    tree::Tree,
    unit::{LayoutType, Rect, Size},
    widget::WidgetId,
};

pub struct CallbackContext<'ui, 'ctx> {
    pub(crate) widget_id: WidgetId,

    pub(crate) tree: &'ctx mut Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,

    pub(crate) widget: &'ctx mut WidgetNode<'ui>,

    pub(crate) notifier: Rc<Notifier>,
}

impl<'ui, 'ctx> CallbackContext<'ui, 'ctx> {
    pub fn get_widget(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_tree(&mut self) -> &Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }

    pub fn mark_dirty(&mut self, listener_id: ListenerId) {
        self.notifier.notify(listener_id);
    }
}

// Globals
impl<'ui, 'ctx> CallbackContext<'ui, 'ctx> {
    /// Initialize a global value if it's not set already. This does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue + Clone,
        F: FnOnce() -> V,
    {
        self.global.get_or(None, func)
    }

    /// Get a global value. This will panic if the global does not exist.
    pub fn get_global<V>(&mut self) -> State<V>
    where
        V: StateValue + Clone,
    {
        self.global.try_get(None).expect("failed to get global")
    }

    /// Set a global value. This does not cause the initializer to be updated when its value is changed.
    pub fn set_global<V>(&mut self, value: V)
    where
        V: StateValue + Clone,
    {
        self.global.set(value)
    }
}

// Local state
impl<'ui, 'ctx> CallbackContext<'ui, 'ctx> {
    /// Initializing a state does not cause the initializer to be updated when its value is changed.
    pub fn init_state<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue + Clone,
        F: FnOnce() -> V,
    {
        self.widget.state.get_or::<V, F>(None, func)
    }

    /// Get the state of the widget. This will panic if the state does not exist.
    pub fn get_state<V>(&mut self) -> State<V>
    where
        V: StateValue + Clone,
    {
        self.widget
            .state
            .try_get(None)
            .expect("failed to get state")
    }

    /// Set the state of the widget.
    pub fn set_state<V>(&mut self, value: V)
    where
        V: StateValue + Clone,
    {
        self.widget.state.set(value)
    }
}

// Layout
impl<'ui, 'ctx> CallbackContext<'ui, 'ctx> {
    /// Fetch the layout of a widget.
    pub fn get_layout_type(&self) -> LayoutType {
        self.widget.layout_type
    }

    pub fn get_rect(&self) -> Option<Rect> {
        self.widget.rect
    }

    pub fn get_size(&self) -> Option<Size> {
        self.widget.rect.map(|rect| rect.into())
    }
}
