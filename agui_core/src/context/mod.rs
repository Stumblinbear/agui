use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::{
    context::state::{Ref, StateMap, Value, WidgetStates},
    widget::{BuildResult, Widget, WidgetID},
};

mod state;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ListenerID {
    Widget(WidgetID),
    Computed(WidgetID, TypeId),
}

impl ListenerID {
    pub fn widget_id(&self) -> &WidgetID {
        match self {
            ListenerID::Widget(widget_id) => widget_id,
            ListenerID::Computed(widget_id, _) => widget_id,
        }
    }
}

impl From<WidgetID> for ListenerID {
    fn from(widget_id: WidgetID) -> Self {
        ListenerID::Widget(widget_id)
    }
}

pub struct WidgetContext {
    global: StateMap,
    states: WidgetStates,

    computed_funcs:
        Arc<Mutex<HashMap<WidgetID, HashMap<TypeId, Box<dyn Fn(&WidgetContext) -> bool>>>>>,

    current_id: Arc<Mutex<Option<ListenerID>>>,
}

impl WidgetContext {
    pub fn new(on_changed: Box<dyn Fn(&HashSet<ListenerID>) + Send + Sync>) -> Self {
        let on_changed = Arc::new(on_changed);

        Self {
            global: StateMap::new(on_changed.clone()),

            states: WidgetStates::new(on_changed.clone()),

            computed_funcs: Arc::new(Mutex::new(HashMap::new())),

            current_id: Default::default(),
        }
    }

    pub fn init_global<V>(&self) -> Ref<V>
    where
        V: Value + Default,
    {
        self.set_global(V::default())
    }

    pub fn set_global<V>(&self, value: V) -> Ref<V>
    where
        V: Value,
    {
        self.global.insert(value);

        self.get_global()
            .expect("failed to fetch global, this shouldn't be possible")
    }

    pub fn get_global<V>(&self) -> Option<Ref<V>>
    where
        V: Value,
    {
        if let Some(listener_id) = *self.current_id.lock() {
            self.global.add_listener(listener_id);
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
        self.get_state_or(|| V::default())
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

    pub fn computed<V, F>(&self, func: F) -> V
    where
        V: Eq + PartialEq + Clone + Value,
        F: Fn(&WidgetContext) -> V + 'static,
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

        if !widgets.contains_key(widget_id) {
            widgets.insert(*widget_id, HashMap::default());
        }

        let computed_funcs = widgets.get_mut(widget_id).unwrap();

        if !computed_funcs.contains_key(&computed_id) {
            let last_value = Arc::new(Mutex::new(value.clone()));

            computed_funcs.insert(
                computed_id,
                Box::new(move |ctx| {
                    let new_value = ctx.call_computed(listener_id, &func);

                    let mut last_value = last_value.lock();

                    if *last_value == new_value {
                        return false;
                    } else {
                        *last_value = new_value;
                        return true;
                    }
                }),
            );
        }

        value
    }

    fn call_computed<V, F>(&self, listener_id: ListenerID, func: &F) -> V
    where
        V: Eq + PartialEq + Clone + Value,
        F: Fn(&WidgetContext) -> V + 'static,
    {
        let previous_id = self.current_id.lock().clone();

        *self.current_id.lock() = Some(listener_id);

        let value = func(self);

        *self.current_id.lock() = previous_id;

        value
    }

    pub(crate) fn did_computed_change(&mut self, widget_id: &WidgetID, computed_id: TypeId) -> bool {
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

    pub(crate) fn build(&mut self, widget_id: WidgetID, widget: &Box<dyn Widget>) -> BuildResult {
        *self.current_id.lock() = Some(widget_id.into());

        widget.build(self)
    }

    pub(crate) fn remove(&mut self, widget_id: &WidgetID) {
        let listener_id = ListenerID::Widget(*widget_id);

        self.global.remove_listener(&listener_id);

        self.states.remove(widget_id);

        self.computed_funcs.lock().remove(&widget_id);
    }
}
