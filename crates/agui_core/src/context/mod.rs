use std::{any::TypeId, rc::Rc, sync::Arc};

use fnv::{FnvHashMap, FnvHashSet};
use parking_lot::Mutex;

mod computed;
mod notifiable;

use self::{
    computed::{ComputedFn, ComputedFunc},
    notifiable::readable::ReadableMap,
};

pub use self::notifiable::{
    state::{ScopedStateMap, StateMap},
    NotifiableValue, Notify,
};

use crate::{
    layout::Layout,
    node::WidgetNode,
    plugin::WidgetPlugin,
    tree::Tree,
    unit::{Key, LayoutType, Rect, Shape},
    widget::{WidgetId, WidgetRef},
    Ref,
};

/// A combined-type for anything that can listen for events in the system.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ListenerId {
    Widget(WidgetId),
    Computed(WidgetId, TypeId),
    Plugin(TypeId),
}

impl ListenerId {
    /// Returns `None` if not tied to a widget.
    #[must_use]
    pub fn widget_id(&self) -> Option<WidgetId> {
        match self {
            Self::Widget(widget_id) | Self::Computed(widget_id, _) => Some(*widget_id),
            Self::Plugin(_) => None,
        }
    }

    /// Attempts to convert listeners into their base widget listener, otherwise makes no changes.
    #[must_use]
    pub fn prefer_widget(&self) -> Self {
        match self {
            Self::Computed(widget_id, _) => Self::Widget(*widget_id),
            _ => *self,
        }
    }
}

type WidgetComputedFuncs<'ui> =
    FnvHashMap<WidgetId, FnvHashMap<TypeId, Box<dyn ComputedFunc<'ui> + 'ui>>>;

pub struct WidgetContext<'ui> {
    pub(crate) tree: Tree<WidgetId, WidgetNode>,
    pub(crate) plugins: Mutex<FnvHashMap<TypeId, Rc<dyn WidgetPlugin>>>,

    global: StateMap,
    states: ScopedStateMap<ListenerId>,

    computed_funcs: Arc<Mutex<WidgetComputedFuncs<'ui>>>,

    layout_types: Mutex<FnvHashMap<WidgetId, Ref<LayoutType>>>,
    layouts: Mutex<FnvHashMap<WidgetId, Ref<Layout>>>,
    clipping: Mutex<FnvHashMap<WidgetId, Ref<Shape>>>,

    pub(crate) rects: ReadableMap<WidgetId, Rect>,

    changed: Arc<Mutex<FnvHashSet<ListenerId>>>,

    pub(crate) current_id: Arc<Mutex<Option<ListenerId>>>,
}

impl<'ui> WidgetContext<'ui> {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(changed: Arc<Mutex<FnvHashSet<ListenerId>>>) -> Self {
        Self {
            tree: Tree::default(),

            plugins: Mutex::default(),

            global: StateMap::new(Arc::clone(&changed)),
            states: ScopedStateMap::new(Arc::clone(&changed)),

            computed_funcs: Arc::new(Mutex::new(FnvHashMap::default())),

            layouts: Mutex::default(),
            layout_types: Mutex::default(),
            clipping: Mutex::default(),

            rects: ReadableMap::new(Arc::clone(&changed)),

            changed,

            current_id: Arc::default(),
        }
    }

    /// Returns the widget tree.
    pub fn get_tree(&self) -> &Tree<WidgetId, WidgetNode> {
        &self.tree
    }

    pub fn get_self(&self) -> ListenerId {
        self.current_id
            .lock()
            .expect("cannot get self while not iterating")
    }

    pub fn is_self(&self, listener_id: ListenerId) -> bool {
        self.get_self() == listener_id
    }

    pub fn mark_dirty(&self, listener_id: ListenerId) {
        self.changed.lock().insert(listener_id);
    }

    pub(crate) fn remove_widget(&mut self, widget_id: WidgetId) {
        let mut all_listeners = vec![ListenerId::Widget(widget_id)];

        if let Some(computed_funcs) = self.computed_funcs.lock().remove(&widget_id) {
            for type_id in computed_funcs.into_keys() {
                all_listeners.push(ListenerId::Computed(widget_id, type_id));
            }
        }

        self.layouts.lock().remove(&widget_id);

        self.rects.remove(&widget_id);

        for listener_id in all_listeners {
            self.remove_listener(listener_id);
        }
    }

    pub(crate) fn remove_listener(&mut self, listener_id: ListenerId) {
        self.global.remove_listeners(&listener_id);

        self.states.remove(&listener_id);

        self.states.remove_listeners(&listener_id);

        self.rects.remove_listeners(&listener_id);
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
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        self.global.get_or(func)
    }

    /// Fetch a global value if it exists. The caller will be updated when the value is changed.
    pub fn try_use_global<V>(&self) -> Option<Notify<V>>
    where
        V: NotifiableValue,
    {
        if let Some(listener_id) = *self.current_id.lock() {
            self.global.add_listener::<V>(listener_id);
        }

        self.global.get::<V>()
    }

    /// Fetch a global value, or initialize it with `func`. The caller will be updated when the value is changed.
    pub fn use_global<V, F>(&self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        if let Some(listener_id) = *self.current_id.lock() {
            self.global.add_listener::<V>(listener_id);
        }

        self.global.get_or(func)
    }
}

// Local state
impl<'ui> WidgetContext<'ui> {
    /// Initializing a state does not cause the initializer to be updated when its value is changed.
    pub fn init_state<V, F>(&self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating")
            .prefer_widget();

        self.states.get(current_id, func)
    }

    /// Fetch a local state value, or initialize it with `func` if it doesn't exist. The caller will be updated when the value is changed.
    pub fn use_state<V, F>(&self, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        self.states
            .add_listener::<V>(current_id.prefer_widget(), current_id);

        self.states.get(current_id.prefer_widget(), func)
    }

    pub fn get_state_for<V, F>(&self, listener_id: ListenerId, func: F) -> Notify<V>
    where
        V: NotifiableValue,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating")
            .prefer_widget();

        self.states.add_listener::<V>(listener_id, current_id);

        self.states.get(listener_id, func)
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
    pub fn get_layout_type(&self, widget_id: WidgetId) -> Ref<LayoutType> {
        self.layout_types
            .lock()
            .get(&widget_id)
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
    pub fn get_layout(&self, widget_id: WidgetId) -> Ref<Layout> {
        self.layouts
            .lock()
            .get(&widget_id)
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
    pub fn get_clipping(&self, widget_id: WidgetId) -> Ref<Shape> {
        self.clipping
            .lock()
            .get(&widget_id)
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
        V: Eq + PartialEq + Clone + NotifiableValue,
        F: Fn(&Self) -> V + 'ui + 'static,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        let widget_id = current_id
            .widget_id()
            .expect("cannot create a computed value outside of a widget");

        let computed_id = TypeId::of::<F>();

        let listener_id = ListenerId::Computed(widget_id, computed_id);

        let mut widgets = self.computed_funcs.lock();

        let computed_func = widgets
            .entry(widget_id)
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
            .expect("failed to downcast ref")
    }

    pub(crate) fn call_computed_func(&mut self, widget_id: WidgetId, computed_id: TypeId) -> bool {
        self.computed_funcs
            .lock()
            .get_mut(&widget_id)
            .and_then(|widgets| widgets.get_mut(&computed_id))
            .map_or(false, |computed_func| computed_func.call(self))
    }
}

// Computed
impl<'ui> WidgetContext<'ui> {
    /// Get the visual `Rect` of a widget.
    pub fn get_rect_for(&self, widget_id: WidgetId) -> Option<&Rect> {
        self.rects.get(&widget_id)
    }

    /// Get to the visual rect of the widget.
    pub fn get_rect(&self) -> Option<&Rect> {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get rect from context while not iterating");

        self.rects.get(
            &current_id
                .widget_id()
                .expect("cannot get rect outside of a widget context"),
        )
    }

    /// Listen to the visual rect of the widget.
    pub fn use_rect(&self) -> Option<&Rect> {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get rect from context while not iterating");

        let widget_id = current_id
            .widget_id()
            .expect("cannot get rect outside of a widget context");

        self.rects.add_listener(widget_id, current_id);

        self.rects.get(&widget_id)
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

        let widget_id = current_id
            .widget_id()
            .expect("cannot create a key outside of a widget");

        WidgetRef::Keyed {
            owner_id: match key {
                Key::Local(_) => Some(widget_id),
                Key::Global(_) => None,
            },
            key,
            widget: Box::new(widget),
        }
    }
}
