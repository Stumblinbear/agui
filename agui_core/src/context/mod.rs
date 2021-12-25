use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use parking_lot::Mutex;

mod state;

pub use self::state::Ref;

use self::state::{StateMap, Value, WidgetStates};

use crate::{
    layout::LayoutRef,
    unit::Key,
    widget::{BuildResult, WidgetId, WidgetRef},
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

type ComputedFn = Box<dyn Fn(&WidgetContext) -> bool>;
type WidgetComputedFuncs = HashMap<WidgetId, HashMap<TypeId, ComputedFn>>;

pub struct WidgetContext {
    global: StateMap,
    states: WidgetStates,

    layouts: Mutex<HashMap<WidgetId, LayoutRef>>,

    computed_funcs: Arc<Mutex<WidgetComputedFuncs>>,

    pub(crate) current_id: Arc<Mutex<Option<ListenerID>>>,
}

impl WidgetContext {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(on_changed: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>) -> Self {
        Self {
            global: StateMap::new(Arc::clone(&on_changed)),
            states: WidgetStates::new(Arc::clone(&on_changed)),
            layouts: Mutex::default(),

            computed_funcs: Arc::new(Mutex::new(HashMap::new())),

            current_id: Arc::default(),
        }
    }

    // Global and local state

    /// Initializing a global does not cause the initializer to be updated when its value is changed.
    pub fn init_global<V>(&self) -> Ref<V>
    where
        V: Value + Default,
    {
        if !self.global.contains::<V>() {
            self.global.insert(V::default());
        }

        self.global.get::<V>().expect("failed to init global")
    }

    pub fn set_global<V>(&self, value: V) -> Ref<V>
    where
        V: Value,
    {
        self.global.insert(value);

        self.get_global()
            .expect("failed to fetch global, this shouldn't be possible")
    }

    pub fn get_or_init_global<V>(&self) -> Ref<V>
    where
        V: Value + Default,
    {
        self.get_global::<V>()
            .map_or_else(|| self.set_global(V::default()), |v| v)
    }

    pub fn get_global<V>(&self) -> Option<Ref<V>>
    where
        V: Value,
    {
        if let Some(listener_id) = *self.current_id.lock() {
            self.global.add_listener::<V>(listener_id);
        }

        self.global.get::<V>()
    }

    pub fn set_state<V>(&self, value: V) -> Ref<V>
    where
        V: Value,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        self.states.set(&current_id, value)
    }

    pub fn get_state<V: Default>(&self) -> Ref<V>
    where
        V: Value,
    {
        self.get_state_or(V::default)
    }

    pub fn get_state_or<V, F>(&self, func: F) -> Ref<V>
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

    pub fn set_layout(&self, layout: LayoutRef) {
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

    pub fn get_layout(&self, widget_id: &WidgetId) -> LayoutRef {
        self.layouts
            .lock()
            .get(widget_id)
            .map_or(LayoutRef::None, LayoutRef::clone)
    }

    // Computed

    /// # Panics
    ///
    /// Will panic if called outside of a build context.
    pub fn computed<V, F>(&self, func: F) -> V
    where
        V: Eq + PartialEq + Clone + Value,
        F: Fn(&Self) -> V + 'static,
    {
        let current_id = self
            .current_id
            .lock()
            .expect("cannot get state from context while not iterating");

        let widget_id = current_id.widget_id();

        let computed_id = TypeId::of::<F>();

        let listener_id = ListenerID::Computed(*widget_id, computed_id);

        let value = self.call_computed(listener_id, &func);

        let mut widgets = self.computed_funcs.lock();

        widgets
            .entry(*widget_id)
            .or_insert_with(HashMap::default)
            .entry(computed_id)
            .or_insert_with(|| {
                let last_value = Arc::new(Mutex::new(value.clone()));

                Box::new(move |ctx| {
                    let new_value = ctx.call_computed(listener_id, &func);

                    let mut last_value = last_value.lock();

                    if *last_value == new_value {
                        false
                    } else {
                        *last_value = new_value;
                        true
                    }
                })
            });

        value
    }

    fn call_computed<V, F>(&self, listener_id: ListenerID, func: &F) -> V
    where
        V: Eq + PartialEq + Clone + Value,
        F: Fn(&Self) -> V + 'static,
    {
        let previous_id = *self.current_id.lock();

        *self.current_id.lock() = Some(listener_id);

        let value = func(self);

        *self.current_id.lock() = previous_id;

        value
    }

    pub(crate) fn did_computed_change(
        &mut self,
        widget_id: &WidgetId,
        computed_id: TypeId,
    ) -> bool {
        let mut widgets = self.computed_funcs.lock();

        if !widgets.contains_key(widget_id) {
            return false;
        }

        let computed_funcs = widgets.get_mut(widget_id).unwrap();

        if !computed_funcs.contains_key(&computed_id) {
            return false;
        }

        (computed_funcs.get(&computed_id).unwrap())(self)
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
