use std::{
    any::TypeId,
    collections::{BTreeMap, HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use parking_lot::Mutex;

mod cache;
mod computed;
mod notifiable;
pub mod tree;
mod value;

pub use self::notifiable::{NotifiableMap, Notify, ScopedNotifiableMap};
pub use self::value::Value;
use self::{
    cache::LayoutCache,
    computed::{ComputedFn, ComputedFunc},
    tree::Tree,
};

use crate::{
    event::WidgetEvent,
    layout::Layout,
    plugin::WidgetPlugin,
    unit::{Key, Rect},
    widget::{BuildResult, WidgetId, WidgetRef},
    Ref,
};

/// A combined-type for anything that can listen for events in the system.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ListenerID {
    Widget(WidgetId),
    Computed(WidgetId, TypeId),
    Plugin(TypeId),
}

impl ListenerID {
    /// # Panics
    ///
    /// Will panic if called on a plugin listener.
    #[must_use]
    pub fn widget_id(&self) -> &WidgetId {
        match self {
            Self::Widget(widget_id) | Self::Computed(widget_id, _) => widget_id,
            ListenerID::Plugin(_) => panic!("listener is not a widget"),
        }
    }
}

type WidgetComputedFuncs<'ui> =
    HashMap<WidgetId, HashMap<TypeId, Box<dyn ComputedFunc<'ui> + 'ui>>>;

pub struct WidgetContext<'ui> {
    pub(crate) tree: Tree<WidgetId>,
    pub(crate) cache: LayoutCache<WidgetId>,

    plugins: Mutex<BTreeMap<TypeId, Rc<dyn WidgetPlugin>>>,

    global: NotifiableMap,
    states: ScopedNotifiableMap<WidgetId>,

    layouts: Mutex<HashMap<WidgetId, Ref<Layout>>>,

    computed_funcs: Arc<Mutex<WidgetComputedFuncs<'ui>>>,

    current_id: Arc<Mutex<Option<ListenerID>>>,
}

impl<'ui> WidgetContext<'ui> {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(changed: Arc<Mutex<HashSet<ListenerID>>>) -> Self {
        Self {
            tree: Tree::default(),
            cache: LayoutCache::default(),

            plugins: Mutex::default(),

            global: NotifiableMap::new(Arc::clone(&changed)),
            states: ScopedNotifiableMap::new(Arc::clone(&changed)),
            layouts: Mutex::default(),

            computed_funcs: Arc::new(Mutex::new(HashMap::new())),

            current_id: Arc::default(),
        }
    }

    /// Returns the widget tree.
    pub fn get_tree(&self) -> &Tree<WidgetId> {
        &self.tree
    }

    /// Get the visual `Rect` of a widget.
    pub fn get_rect(&self, widget_id: &WidgetId) -> Option<&Rect> {
        self.cache.get_rect(widget_id)
    }

    // Plugins

    /// Initialize a plugin if it's not set already. This does not cause the initializer to be updated when it is changed.
    pub fn init_plugin<P>(&self) -> Rc<P>
    where
        P: WidgetPlugin + Default,
    {
        if self.plugins.lock().contains_key(&TypeId::of::<P>()) {
            self.get_plugin::<P>()
                .expect("failed to get initialized plugin")
        } else {
            self.set_plugin(P::default())
        }
    }

    /// Load a plugin, or overwrite one if it already exists.
    pub fn set_plugin<P>(&self, plugin: P) -> Rc<P>
    where
        P: WidgetPlugin,
    {
        let plugin_id = TypeId::of::<P>();

        self.plugins.lock().insert(plugin_id, Rc::new(plugin));

        // Force a first-update of the plugin
        self.plugin_on_update(plugin_id);

        self.get_plugin::<P>().expect("failed to set plugin")
    }

    /// Fetch a plugin, or initialize it if it doesn't exist. The caller will be updated when the plugin is changed.
    pub fn get_or_init_plugin<P>(&self) -> Rc<P>
    where
        P: WidgetPlugin + Default,
    {
        self.get_plugin_or(P::default)
    }

    /// Fetch a plugin. The caller will be updated when the plugin is changed.
    pub fn get_plugin_or<P, F>(&self, func: F) -> Rc<P>
    where
        P: WidgetPlugin,
        F: FnOnce() -> P,
    {
        self.get_plugin::<P>()
            .map_or_else(|| self.set_plugin(func()), |plugin| plugin)
    }

    /// Fetch a plugin. The caller will be updated when the plugin is changed.
    pub fn get_plugin<P>(&self) -> Option<Rc<P>>
    where
        P: WidgetPlugin,
    {
        match Rc::clone(self.plugins.lock().get(&TypeId::of::<P>())?).downcast_rc::<P>() {
            Ok(plugin) => Some(plugin),
            Err(..) => None,
        }
    }

    // Global state

    /// Initialize a global value if it's not set already. This does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V>(&self) -> Notify<V>
    where
        V: Value + Default,
    {
        if !self.global.contains::<V>() {
            self.global.insert(V::default());
        }

        self.global.get::<V>().expect("failed to init global")
    }

    /// Set a global state value. This does not cause the setter to be updateed when its value is changed.
    pub fn set_global<V>(&self, value: V) -> Notify<V>
    where
        V: Value,
    {
        self.global.insert(value);

        self.global.get::<V>().expect("failed to init global")
    }

    /// Fetch a global value, or initialize it if it doesn't exist. The caller will be updated when the value is changed.
    pub fn get_or_init_global<V>(&self) -> Notify<V>
    where
        V: Value + Default,
    {
        self.get_global_or(V::default)
    }

    /// Fetch a global value. The caller will be updated when the value is changed.
    pub fn get_global_or<V, F>(&self, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        self.get_global::<V>()
            .map_or_else(|| self.set_global(func()), |v| v)
    }

    /// Fetch a global value. The caller will be updated when the value is changed.
    pub fn get_global<V>(&self) -> Option<Notify<V>>
    where
        V: Value,
    {
        if let Some(listener_id) = *self.current_id.lock() {
            self.global.add_listener::<V>(listener_id);
        }

        self.global.get::<V>()
    }

    // Local state

    /// Initializing a state does not cause the initializer to be updated when its value is changed.
    pub fn init_state<V>(&self) -> Notify<V>
    where
        V: Value + Default,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        self.states.init(*current_id.widget_id(), V::default)
    }

    /// Set a local state value. This does not cause the initializer to be updated when its value is changed.
    pub fn set_state<V>(&self, value: V) -> Notify<V>
    where
        V: Value,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        self.states.set(*current_id.widget_id(), value)
    }

    /// Fetch a local state value, or initialize it with `func` if it doesn't exist. The caller will be updated when the value is changed.
    pub fn get_state_or<V, F>(&self, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        let widget_id = *current_id.widget_id();

        let state = self.states.get(widget_id, func);

        self.states.add_listener::<V>(widget_id, current_id);

        state
    }

    /// Fetch a local state value. The caller will be updated when the value is changed.
    pub fn get_state<V>(&self) -> Notify<V>
    where
        V: Value + Default,
    {
        self.get_state_or(V::default)
    }

    pub fn get_state_for<V, F>(&self, widget_id: WidgetId, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        let state = self.states.get(widget_id, func);

        self.states.add_listener::<V>(widget_id, current_id);

        state
    }

    // Layout

    /// Set the layout of the widget.
    ///
    /// Used in a `build()` method to set the layout of the widget being built.
    pub fn set_layout(&self, layout: Ref<Layout>) {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        match &current_id {
            ListenerID::Widget(widget_id) => {
                self.layouts.lock().insert(*widget_id, layout);
            }
            ListenerID::Computed(_, _) => {
                log::warn!("layouts set in a computed function are ignored");
            }
            ListenerID::Plugin(_) => {
                log::warn!("layouts set in a plugin are ignored");
            }
        };
    }

    /// Fetch the layout of a widget.
    pub fn get_layout(&self, widget_id: &WidgetId) -> Ref<Layout> {
        self.layouts
            .lock()
            .get(widget_id)
            .map_or(Ref::None, Ref::clone)
    }

    // Computed

    /// # Panics
    ///
    /// Will panic if called outside of a build context.
    pub fn computed<V, F>(&self, func: F) -> V
    where
        V: Eq + PartialEq + Copy + Value,
        F: Fn(&Self) -> V + 'ui + 'static,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        let widget_id = current_id.widget_id();

        let computed_id = TypeId::of::<F>();

        let listener_id = ListenerID::Computed(*widget_id, computed_id);

        let mut widgets = self.computed_funcs.lock();

        let computed_func = widgets
            .entry(*widget_id)
            .or_insert_with(HashMap::default)
            .entry(computed_id)
            .or_insert_with(|| {
                let mut computed_func = Box::new(ComputedFn::new(listener_id, func));

                computed_func.call(self);

                computed_func
            });

        *computed_func
            .get()
            .downcast()
            .ok()
            .expect("failed to downcast ref")
    }

    pub(crate) fn call_computed_func(&mut self, widget_id: &WidgetId, computed_id: TypeId) -> bool {
        self.computed_funcs
            .lock()
            .get_mut(widget_id)
            .and_then(|widgets| widgets.get_mut(&computed_id))
            .map_or(false, |computed_func| computed_func.call(self))
    }

    // Keys

    /// # Panics
    ///
    /// Will panic if called outside of a widget build context.
    pub fn key(&self, key: Key, widget: WidgetRef) -> WidgetRef {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot key from context while not iterating");

        let widget_id = current_id.widget_id();

        WidgetRef::Keyed {
            owner_id: match key {
                Key::Unique(_) | Key::Local(_) => Some(*widget_id),
                Key::Global(_) => None,
            },
            key,
            widget: Box::new(widget),
        }
    }

    // Other

    /// # Panics
    ///
    /// Will panic if called outside of a widget build context, or in a plugin.
    pub fn get_self(&self) -> WidgetId {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get self while not iterating");

        match current_id {
            ListenerID::Widget(widget_id) | ListenerID::Computed(widget_id, _) => widget_id,
            ListenerID::Plugin(_) => {
                panic!("plugins do not exist in the tree, and thus they cannot get themselves")
            }
        }
    }

    /// # Panics
    ///
    /// Will panic if called outside of a widget build context, or in a plugin.
    pub fn is_self(&self, widget_id: WidgetId) -> bool {
        self.get_self() == widget_id
    }

    pub(crate) fn plugin_on_update(&self, plugin_id: TypeId) {
        let plugins = self.plugins.lock();

        let plugin = plugins
            .get(&plugin_id)
            .expect("cannot update a plugin that does not exist");

        let last_id = *self.current_id.lock();

        *self.current_id.lock() = Some(ListenerID::Plugin(plugin_id));

        plugin.on_update(self);

        *self.current_id.lock() = last_id;
    }

    pub(crate) fn plugin_on_events(&self, events: &[WidgetEvent]) {
        let plugins = self.plugins.lock();

        let last_id = *self.current_id.lock();
    
        for (plugin_id, plugin) in plugins.iter() {
            *self.current_id.lock() = Some(ListenerID::Plugin(*plugin_id));

            plugin.on_events(self, events);
        }

        *self.current_id.lock() = last_id;
    }

    pub(crate) fn build(&self, widget_id: WidgetId, widget: &WidgetRef) -> BuildResult {
        let last_id = *self.current_id.lock();

        *self.current_id.lock() = Some(ListenerID::Widget(widget_id));

        let result = widget
            .try_get()
            .map_or(BuildResult::Empty, |widget| widget.build(self));

        *self.current_id.lock() = last_id;

        result
    }

    pub(crate) fn remove(&self, widget_id: &WidgetId) {
        let listener_id = ListenerID::Widget(*widget_id);

        self.global.remove_listener(&listener_id);

        self.states.remove(widget_id);

        self.states.remove_listeners(&listener_id);

        self.layouts.lock().remove(widget_id);

        self.computed_funcs.lock().remove(widget_id);
    }
}
