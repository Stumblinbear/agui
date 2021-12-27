use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use parking_lot::Mutex;

mod computed;
mod state;
mod value;

pub use self::state::State;
pub use self::value::Value;
use self::{
    computed::{ComputedFn, ComputedFunc},
    state::{StateMap, WidgetStates},
};

use crate::{
    layout::Layout,
    unit::Key,
    widget::{BuildResult, WidgetId, WidgetRef},
    Ref,
};

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
    global: StateMap,
    states: WidgetStates,

    layouts: Mutex<HashMap<WidgetId, Ref<Layout>>>,

    computed_funcs: Arc<Mutex<WidgetComputedFuncs<'ui>>>,

    pub(crate) current_id: Arc<Mutex<Option<ListenerID>>>,
}

impl<'ui> WidgetContext<'ui> {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(changed: Arc<Mutex<HashSet<ListenerID>>>) -> Self {
        Self {
            global: StateMap::new(Arc::clone(&changed)),
            states: WidgetStates::new(Arc::clone(&changed)),
            layouts: Mutex::default(),

            computed_funcs: Arc::new(Mutex::new(HashMap::new())),

            current_id: Arc::default(),
        }
    }

    // Global and local state

    /// Initializing a global does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V>(&self) -> State<V>
    where
        V: Value + Default,
    {
        if !self.global.contains::<V>() {
            self.global.insert(V::default());
        }

        self.global.get::<V>().expect("failed to init global")
    }

    pub fn set_global<V>(&self, value: V) -> State<V>
    where
        V: Value,
    {
        self.global.insert(value);

        self.get_global()
            .expect("failed to fetch global, this shouldn't be possible")
    }

    pub fn get_or_init_global<V>(&self) -> State<V>
    where
        V: Value + Default,
    {
        self.get_global::<V>()
            .map_or_else(|| self.set_global(V::default()), |v| v)
    }

    pub fn get_global<V>(&self) -> Option<State<V>>
    where
        V: Value,
    {
        if let Some(listener_id) = *self.current_id.lock() {
            self.global.add_listener::<V>(listener_id);
        }

        self.global.get::<V>()
    }

    pub fn set_state<V>(&self, value: V) -> State<V>
    where
        V: Value,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        self.states.set(&current_id, value)
    }

    pub fn get_state<V: Default>(&self) -> State<V>
    where
        V: Value,
    {
        self.get_state_or(V::default)
    }

    pub fn get_state_or<V, F>(&self, func: F) -> State<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        self.states.get(&current_id, func)
    }

    // Layout

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
        V: Eq + PartialEq + Copy + Clone + Value,
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

    pub(crate) fn build(&mut self, widget_id: WidgetId, widget: &WidgetRef) -> BuildResult {
        *self.current_id.lock() = Some(ListenerID::Widget(widget_id));

        let result = widget
            .try_get()
            .map_or(BuildResult::Empty, |widget| widget.build(self));

        *self.current_id.lock() = None;

        result
    }

    pub(crate) fn remove(&mut self, widget_id: &WidgetId) {
        let listener_id = ListenerID::Widget(*widget_id);

        self.global.remove_listener(&listener_id);

        self.states.remove(widget_id);

        self.layouts.lock().remove(widget_id);

        self.computed_funcs.lock().remove(widget_id);
    }
}
