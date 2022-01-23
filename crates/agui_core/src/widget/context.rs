use crate::{
    canvas::painter::CanvasPainter,
    computed::{ComputedContext, ComputedFn, ComputedFunc, ComputedId},
    engine::node::WidgetNode,
    notifiable::{state::StateMap, NotifiableValue, Notify},
    tree::Tree,
    unit::{Key, Layout, LayoutType, Ref},
    widget::{WidgetId, WidgetRef},
};

pub struct WidgetContext<'ui, 'ctx> {
    pub(crate) widget_id: WidgetId,
    pub(crate) widget: &'ctx mut WidgetNode<'ui>,

    pub(crate) tree: &'ctx mut Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,
}

impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    pub fn get_widget(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_tree(&mut self) -> &Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }

    pub fn get_tree_mut(&mut self) -> &mut Tree<WidgetId, WidgetNode<'ui>> {
        self.tree
    }
}

// Globals
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    /// Fetch a global value if it exists. The caller will be updated when the value is changed.
    pub fn try_use_global<V>(&mut self) -> Option<Notify<V>>
    where
        V: NotifiableValue,
    {
        self.global.add_listener::<V>(self.widget_id.into());

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
        self.global.add_listener::<V>(self.widget_id.into());

        self.global.get_or(func)
    }
}

// Local state
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
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
        self.widget.state.add_listener::<V>(self.widget_id.into());

        self.widget.state.get_or::<V, F>(func)
    }

    pub fn use_state_from<V, F>(&mut self, widget_id: WidgetId, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        let target_widget = self
            .tree
            .get_mut(widget_id)
            .expect("cannot use state from a widget that doesn't exist");

        target_widget.state.add_listener::<V>(self.widget_id.into());

        target_widget.state.get_or::<V, F>(func)
    }
}

// Computed
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    pub fn computed<V, F>(&mut self, func: F) -> V
    where
        V: Eq + PartialEq + Clone + NotifiableValue,
        F: Fn(&mut ComputedContext<'ui, '_>) -> V + 'ui + 'static,
    {
        let computed_id = ComputedId::of::<F>();

        #[allow(clippy::map_entry)]
        if !self.widget.computed_funcs.contains_key(&computed_id) {
            let mut computed_func = Box::new(ComputedFn::new(func));

            computed_func.call(&mut ComputedContext {
                widget_id: self.widget_id,
                computed_id,

                widget: &mut self.widget,

                tree: self.tree,
                global: &mut self.global,
            });

            self.widget
                .computed_funcs
                .insert(computed_id, computed_func);
        }

        *self
            .widget
            .computed_funcs
            .get(&computed_id)
            .expect("failed to set computed function")
            .get()
            .downcast()
            .expect("failed to downcast ref")
    }
}

// Layout
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
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
}

// Rendering
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    /// Set the painter of the widget.
    pub fn set_painter<P>(&mut self, painter: P)
    where
        P: CanvasPainter + 'static,
    {
        self.widget.painter = Some(Box::new(painter));
    }
}

// Keys
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
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
