use std::{any::TypeId, rc::Rc};

use crate::{
    canvas::{renderer::RenderFn, Canvas},
    engine::{node::WidgetNode, notify::Notifier},
    state::{map::StateMap, ListenerId, State, StateValue},
    tree::Tree,
    unit::{Key, Layout, LayoutType, Rect, Ref, Size},
    widget::{
        callback::{Callback, CallbackFn, CallbackId},
        computed::{ComputedFn, ComputedFunc},
        effect::{EffectFn, EffectFunc},
        WidgetId, WidgetRef,
    },
};

use super::{
    widget::{HandlerId, HandlerType, WidgetContext},
    CallbackContext,
};

pub struct BuildContext<'ui, 'ctx> {
    pub(crate) widget_id: WidgetId,
    pub(crate) widget: &'ctx mut WidgetNode<'ui>,

    pub(crate) tree: &'ctx mut Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,

    pub(crate) notifier: Rc<Notifier>,
}

impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    pub fn get_widget(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_listener(&self) -> ListenerId {
        self.widget_id.into()
    }

    pub fn get_tree(&mut self) -> &Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }
}

// Globals
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    /// Fetch a global value if it exists. The caller will be updated when the value is changed.
    pub fn try_use_global<V>(&mut self) -> Option<State<V>>
    where
        V: StateValue + Clone,
    {
        self.global.try_get::<V>(Some(self.get_listener()))
    }

    /// Initialize a global value if it's not set already. This does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue + Clone,
        F: FnOnce() -> V,
    {
        self.global.get_or(None, func)
    }

    /// Fetch a global value, or initialize it with `func`. The caller will be updated when the value is changed.
    pub fn use_global<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue + Clone,
        F: FnOnce() -> V,
    {
        self.global.get_or(Some(self.get_listener()), func)
    }

    /// Get a global value. This will panic if the global does not exist.
    pub fn get_global<V>(&mut self) -> State<V>
    where
        V: StateValue + Clone,
    {
        self.global.try_get(None).expect("failed to get global")
    }

    /// Set a global value. This does not cause the initializer to be updated when its value is changed.
    pub fn set_global<V>(&mut self, value: V) -> State<V>
    where
        V: StateValue + Clone,
    {
        self.global.set(value)
    }
}

// Local state
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    /// Initializing a state does not cause the initializer to be updated when its value is changed.
    pub fn init_state<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue + Clone,
        F: FnOnce() -> V,
    {
        self.widget.state.get_or::<V, F>(None, func)
    }

    /// Fetch a local state value, or initialize it with `func` if it doesn't exist. The caller will be updated when the value is changed.
    pub fn use_state<V, F>(&mut self, func: F) -> State<V>
    where
        V: StateValue + Clone,
        F: FnOnce() -> V,
    {
        self.widget
            .state
            .get_or::<V, F>(Some(self.get_listener()), func)
    }

    pub fn use_state_from<V, F>(&mut self, widget_id: WidgetId, func: F) -> State<V>
    where
        V: StateValue + Clone,
        F: FnOnce() -> V,
    {
        let listener_id = self.get_listener();

        let target_widget = self
            .tree
            .get_mut(widget_id)
            .expect("cannot use state from a widget that doesn't exist");

        target_widget.state.get_or::<V, F>(Some(listener_id), func)
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

    /// Set the state of the widget. This does not cause the initializer to be updated when its value is changed.
    pub fn set_state<V>(&mut self, value: V) -> State<V>
    where
        V: StateValue + Clone,
    {
        self.widget.state.set(value)
    }
}

// Effects
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    pub fn use_effect<F>(&mut self, func: F)
    where
        F: Fn(&mut WidgetContext<'ui, '_>) + 'ui + 'static,
    {
        let handler_id = HandlerId::of::<F>(HandlerType::Effect);

        #[allow(clippy::map_entry)]
        if !self.widget.effect_funcs.contains_key(&handler_id) {
            let effect_func = Box::new(EffectFn::new(func));

            effect_func.call(&mut WidgetContext {
                widget_id: self.widget_id,
                handler_id,

                tree: self.tree,
                global: &mut self.global,

                widget: &mut self.widget,

                notifier: Rc::clone(&self.notifier),
            });

            self.widget.effect_funcs.insert(handler_id, effect_func);
        }
    }
}

// Computed
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    pub fn computed<V, F>(&mut self, func: F) -> V
    where
        V: Eq + PartialEq + Clone + StateValue,
        F: Fn(&mut WidgetContext<'ui, '_>) -> V + 'ui + 'static,
    {
        let handler_id = HandlerId::of::<F>(HandlerType::Computed);

        #[allow(clippy::map_entry)]
        if !self.widget.computed_funcs.contains_key(&handler_id) {
            let mut computed_func = Box::new(ComputedFn::new(func));

            computed_func.call(&mut WidgetContext {
                widget_id: self.widget_id,
                handler_id,

                tree: self.tree,
                global: &mut self.global,

                widget: &mut self.widget,

                notifier: Rc::clone(&self.notifier),
            });

            self.widget.computed_funcs.insert(handler_id, computed_func);
        }

        *self
            .widget
            .computed_funcs
            .get(&handler_id)
            .expect("failed to set computed function")
            .get()
            .downcast()
            .expect("failed to downcast ref")
    }
}

// Callbacks
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    pub fn use_callback<F, A>(&mut self, func: F) -> Callback<A>
    where
        F: Fn(&mut CallbackContext<'ui, '_>, &A) + 'ui + 'static,
        A: StateValue + Clone,
    {
        let callback_id = CallbackId(self.get_widget(), TypeId::of::<F>());

        let callback = Callback::new(callback_id, Rc::clone(&self.notifier));

        self.widget
            .callback_funcs
            .insert(callback_id, Box::new(CallbackFn::new(func)));

        callback
    }
}

// Layout
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    /// Set the layout type of the widget.
    pub fn set_layout_type(&mut self, layout_type: Ref<LayoutType>) {
        self.widget.layout_type = layout_type;
    }

    /// Fetch the layout of the widget.
    pub fn get_layout_type(&self) -> Ref<LayoutType> {
        Ref::clone(&self.widget.layout_type)
    }

    /// Set the layout of the widget.
    pub fn set_layout(&mut self, layout: Ref<Layout>) {
        self.widget.layout = layout;
    }

    pub fn get_rect(&self) -> Option<Rect> {
        self.widget.rect
    }

    pub fn get_size(&self) -> Option<Size> {
        self.widget.rect.map(|rect| rect.into())
    }
}

// Rendering
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    pub fn on_draw<F>(&mut self, func: F)
    where
        F: Fn(&mut Canvas) + 'ui,
    {
        self.widget.renderer = Some(RenderFn::new(func));
    }
}

// Keys
impl<'ui, 'ctx> BuildContext<'ui, 'ctx> {
    pub fn key(&self, key: Key, widget: WidgetRef) -> WidgetRef {
        WidgetRef::Keyed {
            owner_id: match key {
                Key::Local(_) => Some(self.widget_id),
                Key::Global(_) => None,
            },
            key,
            widget: Box::new(widget),
        }
    }
}
