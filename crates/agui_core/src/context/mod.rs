use std::{any::TypeId, rc::Rc, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};
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
    layout::Layout,
    plugin::WidgetPlugin,
    unit::{Key, LayoutType, Rect, Shape},
    widget::{WidgetId, WidgetRef},
    Ref,
};

/// A combined-type for anything that can listen for events in the system.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ListenerId {
    Widget(WidgetId),
    Computed(WidgetId, TypeId),
    Plugin(TypeId),
}

impl ListenerId {
    /// # Panics
    ///
    /// Will panic if called on a plugin listener.
    #[must_use]
    pub fn widget_id(&self) -> &WidgetId {
        match self {
            Self::Widget(widget_id) | Self::Computed(widget_id, _) => widget_id,
            ListenerId::Plugin(_) => panic!("listener is not a widget"),
        }
    }
}

type WidgetComputedFuncs<'ui> =
    FnvHashMap<WidgetId, FnvHashMap<TypeId, Box<dyn ComputedFunc<'ui> + 'ui>>>;

pub struct WidgetContext<'ui> {
    pub(crate) tree: Tree<WidgetId>,
    pub(crate) cache: LayoutCache<WidgetId>,

    pub(crate) plugins: Mutex<FnvHashMap<TypeId, Rc<dyn WidgetPlugin>>>,

    global: NotifiableMap,
    states: ScopedNotifiableMap<WidgetId>,

    layouts: Mutex<FnvHashMap<WidgetId, Ref<Layout>>>,
    layout_types: Mutex<FnvHashMap<WidgetId, Ref<LayoutType>>>,
    clipping: Mutex<FnvHashMap<WidgetId, Ref<Shape>>>,

    computed_funcs: Arc<Mutex<WidgetComputedFuncs<'ui>>>,

    pub(crate) current_id: Arc<Mutex<Option<ListenerId>>>,
}

impl<'ui> WidgetContext<'ui> {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(changed: Arc<Mutex<FnvHashSet<ListenerId>>>) -> Self {
        Self {
            tree: Tree::default(),
            cache: LayoutCache::default(),

            plugins: Mutex::default(),

            global: NotifiableMap::new(Arc::clone(&changed)),
            states: ScopedNotifiableMap::new(Arc::clone(&changed)),

            layouts: Mutex::default(),
            layout_types: Mutex::default(),
            clipping: Mutex::default(),

            computed_funcs: Arc::new(Mutex::new(FnvHashMap::default())),

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

    /// # Panics
    ///
    /// Will panic if called outside of a widget build context, or in a plugin.
    pub fn get_self(&self) -> WidgetId {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get self while not iterating");

        match current_id {
            ListenerId::Widget(widget_id) | ListenerId::Computed(widget_id, _) => widget_id,
            ListenerId::Plugin(_) => {
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

    pub(crate) fn remove(&mut self, widget_id: &WidgetId) {
        let listener_id = ListenerId::Widget(*widget_id);

        self.cache.remove(widget_id);

        self.global.remove_listener(&listener_id);

        self.states.remove(widget_id);

        self.states.remove_listeners(&listener_id);

        self.layouts.lock().remove(widget_id);

        self.computed_funcs.lock().remove(widget_id);
    }
}

// Plugins
impl<'ui> WidgetContext<'ui> {
    /// Initialize a plugin if it's not set already.
    pub fn init_plugin<P, F>(&self, func: F) -> Rc<P>
    where
        P: WidgetPlugin,
        F: FnOnce() -> P,
    {
        if self.plugins.lock().contains_key(&TypeId::of::<P>()) {
            self.get_plugin::<P>()
                .expect("failed to get initialized plugin")
        } else {
            let plugin_id = TypeId::of::<P>();

            let plugin = func();

            let last_id = *self.current_id.lock();

            *self.current_id.lock() = Some(ListenerId::Plugin(plugin_id));

            plugin.on_update(self);

            *self.current_id.lock() = last_id;

            self.plugins.lock().insert(plugin_id, Rc::new(plugin));

            self.get_plugin::<P>().expect("failed to set plugin")
        }
    }

    /// Fetch a plugin, or initialize it with `func`.
    pub fn get_plugin_or<P, F>(&self, func: F) -> Rc<P>
    where
        P: WidgetPlugin,
        F: FnOnce() -> P,
    {
        self.get_plugin::<P>()
            .map_or_else(|| self.init_plugin(func), |plugin| plugin)
    }

    /// Fetch a plugin if it exists.
    pub fn get_plugin<P>(&self) -> Option<Rc<P>>
    where
        P: WidgetPlugin,
    {
        match Rc::clone(self.plugins.lock().get(&TypeId::of::<P>())?).downcast_rc::<P>() {
            Ok(plugin) => Some(plugin),
            Err(..) => None,
        }
    }
}

// Globals
impl<'ui> WidgetContext<'ui> {
    /// Initialize a global value if it's not set already. This does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V, F>(&self, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        if !self.global.contains::<V>() {
            self.global.set(func());
        }

        self.global.get::<V>().expect("failed to init global")
    }

    /// Fetch a global value, or initialize it with `func`. The caller will be updated when the value is changed.
    pub fn use_global<V, F>(&self, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        self.try_use_global::<V>()
            .map_or_else(|| self.init_global(func), |v| v)
    }

    /// Fetch a global value if it exists. The caller will be updated when the value is changed.
    pub fn try_use_global<V>(&self) -> Option<Notify<V>>
    where
        V: Value,
    {
        if let Some(listener_id) = *self.current_id.lock() {
            self.global.add_listener::<V>(listener_id);
        }

        self.global.get::<V>()
    }
}

// Local state
impl<'ui> WidgetContext<'ui> {
    /// Initializing a state does not cause the initializer to be updated when its value is changed.
    pub fn init_state<V, F>(&self, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        let widget_id = *current_id.widget_id();

        self.states.get(widget_id, func)
    }

    /// Fetch a local state value, or initialize it with `func` if it doesn't exist. The caller will be updated when the value is changed.
    pub fn use_state<V, F>(&self, func: F) -> Notify<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        let widget_id = *current_id.widget_id();

        self.states.add_listener::<V>(widget_id, current_id);

        self.states.get(widget_id, func)
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

        self.states.add_listener::<V>(widget_id, current_id);

        self.states.get(widget_id, func)
    }
}

// Layout
impl<'ui> WidgetContext<'ui> {
    /// Set the layout type of the widget.
    ///
    /// Used in a `build()` method to set the layout type of the widget being built.
    pub fn set_layout_type(&self, layout_type: Ref<LayoutType>) {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        match &current_id {
            ListenerId::Widget(widget_id) => {
                self.layout_types.lock().insert(*widget_id, layout_type);
            }
            ListenerId::Computed(_, _) => {
                log::warn!("layout types set in a computed function are ignored");
            }
            ListenerId::Plugin(_) => {
                log::warn!("layout types set in a plugin are ignored");
            }
        };
    }

    /// Fetch the layout of a widget.
    pub fn get_layout_type(&self, widget_id: &WidgetId) -> Ref<LayoutType> {
        self.layout_types
            .lock()
            .get(widget_id)
            .map_or(Ref::None, Ref::clone)
    }

    /// Set the layout of the widget.
    ///
    /// Used in a `build()` method to set the layout of the widget being built.
    pub fn set_layout(&self, layout: Ref<Layout>) {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        match &current_id {
            ListenerId::Widget(widget_id) => {
                self.layouts.lock().insert(*widget_id, layout);
            }
            ListenerId::Computed(_, _) => {
                log::warn!("layouts set in a computed function are ignored");
            }
            ListenerId::Plugin(_) => {
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
}

// Clipping
impl<'ui> WidgetContext<'ui> {
    /// Set the clipping mask of the widget.
    ///
    /// Used in a `build()` method to set the clipping mask of the widget being built.
    pub fn set_clipping(&self, clipping: Ref<Shape>) {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        match &current_id {
            ListenerId::Widget(widget_id) => {
                self.clipping.lock().insert(*widget_id, clipping);
            }
            ListenerId::Computed(_, _) => {
                log::warn!("layouts set in a computed function are ignored");
            }
            ListenerId::Plugin(_) => {
                log::warn!("layouts set in a plugin are ignored");
            }
        };
    }

    /// Fetch the clipping mask of a widget.
    pub fn get_clipping(&self, widget_id: &WidgetId) -> Ref<Shape> {
        self.clipping
            .lock()
            .get(widget_id)
            .map_or(Ref::None, Ref::clone)
    }
}

// Computed
impl<'ui> WidgetContext<'ui> {
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

        let listener_id = ListenerId::Computed(*widget_id, computed_id);

        let mut widgets = self.computed_funcs.lock();

        let computed_func = widgets
            .entry(*widget_id)
            .or_insert_with(FnvHashMap::default)
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
}

// Keys
impl<'ui> WidgetContext<'ui> {
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
                Key::Local(_) => Some(*widget_id),
                Key::Global(_) => None,
            },
            key,
            widget: Box::new(widget),
        }
    }
}
