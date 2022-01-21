use crate::{
    canvas::painter::Painter,
    computed::{ComputedContext, ComputedFn, ComputedFunc, ComputedId},
    engine::node::WidgetNode,
    notifiable::{state::StateMap, ListenerId, NotifiableValue, Notify},
    tree::Tree,
    unit::{Key, Layout, LayoutType, Rect, Ref, Shape},
    widget::{WidgetId, WidgetRef},
};

pub struct WidgetContext<'ui, 'ctx> {
    pub(crate) widget_id: WidgetId,
    pub(crate) widget: &'ctx mut WidgetNode<'ui>,

    pub(crate) tree: &'ctx Tree<WidgetId, WidgetNode<'ui>>,
    pub(crate) global: &'ctx mut StateMap,
}

impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    pub fn get_widget(&self) -> WidgetId {
        self.widget_id
    }

    pub fn get_tree(&self) -> &'ctx Tree<WidgetId, WidgetNode<'ui>> {
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

    pub fn use_state_of<V, F>(&mut self, listener_id: ListenerId, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.widget.state.add_listener::<V>(listener_id);

        self.widget.state.get_or::<V, F>(func)
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

    /// Listen to the visual rect of the widget.
    pub fn use_rect(&mut self) -> Option<Rect> {
        self.widget.rect.add_listener(self.widget_id.into());

        if self.widget.rect.has_value() {
            Some(*self.widget.rect.read())
        } else {
            None
        }
    }
}

// Rendering
impl<'ui, 'ctx> WidgetContext<'ui, 'ctx> {
    /// Set the painter of the widget.
    pub fn set_painter<P>(&mut self, painter: P)
    where
        P: Painter + 'static,
    {
        self.widget.painter = Some(Box::new(painter));
    }

    /// Set the clipping mask of the widget.
    pub fn set_clipping(&mut self, clipping: Ref<Shape>) {
        self.widget.clipping = clipping;
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
